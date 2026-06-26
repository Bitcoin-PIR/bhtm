//! MuHash — incremental multiset hash compatible with Bitcoin Core's coinstats path.
//!
//! Bitcoin Core's UTXO set MuHash feeds each unspent coin as:
//!
//!   COutPoint ‖ ((height << 1) | coinbase) ‖ CTxOut
//!
//! where COutPoint is txid + vout, and CTxOut is amount + scriptPubKey using
//! Bitcoin consensus serialization. Core maps each serialized coin to a
//! 3072-bit field element by SHA256(input) -> ChaCha20(key, zero nonce)
//! -> 384-byte little-endian Num3072.
//!
//! Removals are accumulated in a denominator and folded in during finalization,
//! matching Bitcoin Core's `MuHash3072` behavior.

use chacha20::cipher::{KeyIvInit, StreamCipher};
use sha2::{Digest, Sha256};

use crate::num3072::Num3072;

/// A single UTXO entry to be added or removed from MuHash.
#[derive(Clone, Debug)]
pub struct UtxoEntry {
    /// Transaction ID in Bitcoin Core's internal byte order.
    pub txid: [u8; 32],
    /// Output index within the transaction.
    pub vout: u32,
    /// Amount in satoshis.
    pub amount: u64,
    /// The scriptPubKey bytes.
    pub script_pubkey: Vec<u8>,
    /// Block height where this UTXO was created.
    pub height: u32,
    /// Whether this UTXO was created by a coinbase transaction.
    pub is_coinbase: bool,
}

/// The MuHash accumulator.
#[derive(Clone, Debug)]
pub struct MuHash {
    /// Accumulated inserted elements.
    numerator: Num3072,
    /// Accumulated removed elements. Finalization divides by this value.
    denominator: Num3072,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MuHashParts {
    pub numerator: Num3072,
    pub denominator: Num3072,
}

impl MuHash {
    /// Create a new empty MuHash (value = 1, the multiplicative identity).
    pub fn new() -> Self {
        MuHash {
            numerator: Num3072::ONE,
            denominator: Num3072::ONE,
        }
    }

    /// Create a MuHash accumulator from a trusted normalized 384-byte value.
    ///
    /// A 32-byte MuHash commitment is not enough to continue accumulating,
    /// because the commitment is SHA256(value). Continuing from a non-genesis
    /// anchor requires this preimage value.
    pub fn from_normalized_bytes(bytes: &[u8; 384]) -> Self {
        MuHash {
            numerator: Num3072::from_le_bytes(bytes),
            denominator: Num3072::ONE,
        }
    }

    /// Return the normalized current MuHash value as 384 little-endian bytes.
    pub fn to_bytes(&self) -> [u8; 384] {
        self.normalized_value().to_le_bytes()
    }

    pub(crate) fn parts(&self) -> MuHashParts {
        MuHashParts {
            numerator: self.numerator,
            denominator: self.denominator,
        }
    }

    pub(crate) fn normalized_bytes_from_parts(parts: MuHashParts) -> [u8; 384] {
        normalized_value_from_parts(parts).to_le_bytes()
    }

    pub(crate) fn normalized_bytes_batch(parts: &[MuHashParts]) -> Vec<[u8; 384]> {
        let denominators = parts
            .iter()
            .map(|part| part.denominator)
            .collect::<Vec<_>>();
        let inverses = Num3072::batch_mod_inv(&denominators);

        parts
            .iter()
            .zip(inverses)
            .map(|(part, inverse)| {
                if part.denominator.is_one() {
                    part.numerator.to_le_bytes()
                } else {
                    part.numerator.mod_mul(&inverse).to_le_bytes()
                }
            })
            .collect()
    }

    /// Return the Bitcoin Core 32-byte MuHash commitment.
    pub fn commitment(&self) -> [u8; 32] {
        let data = self.to_bytes();
        let digest = Sha256::digest(data);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }

    /// Return the current normalized MuHash value.
    pub fn value(&self) -> &Num3072 {
        // This method is kept for API compatibility with early tests. It can
        // only return the numerator directly when no removals are pending.
        assert!(
            self.denominator.is_one(),
            "MuHash::value requires a normalized accumulator"
        );
        &self.numerator
    }

    /// Add a UTXO to the multiset: MuHash *= H(utxo) mod p.
    pub fn add(&mut self, utxo: &UtxoEntry) {
        let hash = hash_to_element(utxo);
        self.numerator = self.numerator.mod_mul(&hash);
    }

    /// Remove a UTXO from the multiset.
    pub fn remove(&mut self, utxo: &UtxoEntry) {
        let hash = hash_to_element(utxo);
        self.denominator = self.denominator.mod_mul(&hash);
    }

    /// Batch add multiple UTXOs.
    pub fn add_batch(&mut self, utxos: &[UtxoEntry]) {
        for utxo in utxos {
            self.add(utxo);
        }
    }

    /// Batch remove multiple UTXOs.
    pub fn remove_batch(&mut self, utxos: &[UtxoEntry]) {
        for utxo in utxos {
            self.remove(utxo);
        }
    }

    fn normalized_value(&self) -> Num3072 {
        normalized_value_from_parts(self.parts())
    }
}

fn normalized_value_from_parts(parts: MuHashParts) -> Num3072 {
    if parts.denominator.is_one() {
        parts.numerator
    } else {
        let inv = parts.denominator.mod_inv();
        parts.numerator.mod_mul(&inv)
    }
}

impl Default for MuHash {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a UTXO entry to the 3072-bit field element used by Bitcoin Core.
fn hash_to_element(utxo: &UtxoEntry) -> Num3072 {
    let data = serialize_coin(utxo);
    let key = Sha256::digest(data);

    let mut bytes = [0u8; 384];
    let nonce = [0u8; 12];
    let mut cipher = chacha20::ChaCha20::new((&key).into(), (&nonce).into());
    cipher.apply_keystream(&mut bytes);
    Num3072::from_le_bytes(&bytes)
}

/// Serialize a UTXO entry like Bitcoin Core's coinstats `TxOutSer`.
///
/// Format: COutPoint ‖ uint32(height << 1 | coinbase) ‖ CTxOut.
fn serialize_coin(utxo: &UtxoEntry) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 + 4 + 4 + 8 + 9 + utxo.script_pubkey.len());
    data.extend_from_slice(&utxo.txid);
    data.extend_from_slice(&utxo.vout.to_le_bytes());
    let code = (utxo.height << 1) | u32::from(utxo.is_coinbase);
    data.extend_from_slice(&code.to_le_bytes());
    data.extend_from_slice(&utxo.amount.to_le_bytes());
    write_compact_size(&mut data, utxo.script_pubkey.len() as u64);
    data.extend_from_slice(&utxo.script_pubkey);
    data
}

fn write_compact_size(out: &mut Vec<u8>, n: u64) {
    if n < 0xFD {
        out.push(n as u8);
    } else if n <= 0xFFFF {
        out.push(0xFD);
        out.extend_from_slice(&(n as u16).to_le_bytes());
    } else if n <= 0xFFFF_FFFF {
        out.push(0xFE);
        out.extend_from_slice(&(n as u32).to_le_bytes());
    } else {
        out.push(0xFF);
        out.extend_from_slice(&n.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_muhash_add_remove_identity() {
        let utxo = UtxoEntry {
            txid: [0xAA; 32],
            vout: 0,
            amount: 50_0000_0000, // 50 BTC in satoshis
            script_pubkey: vec![
                0x76, 0xa9, 0x14, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
                0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14,
            ], // simplified P2PKH
            height: 1,
            is_coinbase: true,
        };

        let mut muhash = MuHash::new();
        let initial = muhash.to_bytes();

        muhash.add(&utxo);
        assert_ne!(muhash.to_bytes(), initial);

        muhash.remove(&utxo);
        assert_eq!(muhash.to_bytes(), initial);
    }

    #[test]
    fn test_muhash_commutative() {
        let utxo1 = UtxoEntry {
            txid: [0x11; 32],
            vout: 0,
            amount: 100_000,
            script_pubkey: vec![0x51],
            height: 1,
            is_coinbase: false,
        };
        let utxo2 = UtxoEntry {
            txid: [0x22; 32],
            vout: 1,
            amount: 200_000,
            script_pubkey: vec![0x52],
            height: 2,
            is_coinbase: false,
        };

        let mut mh1 = MuHash::new();
        mh1.add(&utxo1);
        mh1.add(&utxo2);

        let mut mh2 = MuHash::new();
        mh2.add(&utxo2);
        mh2.add(&utxo1);

        assert_eq!(mh1.to_bytes(), mh2.to_bytes());
    }

    #[test]
    fn test_muhash_batch_normalization_matches_individual_normalization() {
        let entries: Vec<UtxoEntry> = (1..=6)
            .map(|tag| UtxoEntry {
                txid: [tag; 32],
                vout: tag as u32,
                amount: 100_000 * tag as u64,
                script_pubkey: vec![0x51, tag],
                height: tag as u32,
                is_coinbase: tag % 2 == 0,
            })
            .collect();

        let mut muhash = MuHash::new();
        let mut snapshots = Vec::new();
        let mut expected = Vec::new();

        muhash.add(&entries[0]);
        snapshots.push(muhash.parts());
        expected.push(muhash.to_bytes());

        muhash.add(&entries[1]);
        muhash.add(&entries[2]);
        snapshots.push(muhash.parts());
        expected.push(muhash.to_bytes());

        muhash.remove(&entries[0]);
        snapshots.push(muhash.parts());
        expected.push(muhash.to_bytes());

        muhash.remove(&entries[1]);
        muhash.add(&entries[3]);
        muhash.add(&entries[4]);
        snapshots.push(muhash.parts());
        expected.push(muhash.to_bytes());

        muhash.remove(&entries[2]);
        muhash.remove(&entries[3]);
        muhash.add(&entries[5]);
        snapshots.push(muhash.parts());
        expected.push(muhash.to_bytes());

        assert_eq!(MuHash::normalized_bytes_batch(&snapshots), expected);
    }

    #[test]
    fn test_hash_to_element_deterministic() {
        let utxo = UtxoEntry {
            txid: [0x42; 32],
            vout: 5,
            amount: 1_000_000,
            script_pubkey: b"test-script".to_vec(),
            height: 3,
            is_coinbase: false,
        };
        let h1 = hash_to_element(&utxo);
        let h2 = hash_to_element(&utxo);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_serialize_coin_roundtrip() {
        let utxo = UtxoEntry {
            txid: [0xAB; 32],
            vout: 3,
            amount: 123456789,
            script_pubkey: vec![0x76, 0xa9, 0x14, 0x01, 0x02, 0x03],
            height: 42,
            is_coinbase: true,
        };
        let data = serialize_coin(&utxo);
        // outpoint(36) + code(4) + amount(8) + compact len(1) + scriptPubKey(6) = 55
        assert_eq!(data.len(), 55);
        assert_eq!(&data[0..32], &[0xAB; 32]);
        assert_eq!(u32::from_le_bytes(data[32..36].try_into().unwrap()), 3);
        assert_eq!(u32::from_le_bytes(data[36..40].try_into().unwrap()), 85);
        assert_eq!(
            u64::from_le_bytes(data[40..48].try_into().unwrap()),
            123456789
        );
        assert_eq!(data[48], 6);
        assert_eq!(&data[49..], &[0x76, 0xa9, 0x14, 0x01, 0x02, 0x03]);
    }
}
