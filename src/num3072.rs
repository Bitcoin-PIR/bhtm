//! 3072-bit modular arithmetic for MuHash.
//!
//! Implements a fixed-size 3072-bit integer ([u64; 48]) with fast modular
//! reduction for the pseudo-Mersenne prime:
//!
//!   p = 2^3072 - 1103717
//!
//! This prime comes from RFC 3526's 3072-bit MODP group and is the same one
//! used by Bitcoin Core's MuHash (src/crypto/muhash.cpp).
//!
//! The key insight for fast reduction: for any 6144-bit product x = a * b,
//! write x = high * 2^3072 + low, then:
//!
//!   x mod p = (high * c + low) mod p   where c = 1103717
//!
//! Since c < 2^21, high * c fits in ~3093 bits, so one more conditional
//! subtraction gives the final result.

use std::cmp::Ordering;
use std::fmt;
use std::sync::LazyLock;

/// The MuHash prime: 2^3072 - 1103717
pub fn p() -> Num3072 {
    let mut d = [0u64; 48];
    for i in 0..48 {
        d[i] = 0xFFFFFFFFFFFFFFFF;
    }
    d[0] = d[0].wrapping_sub(1103716);
    Num3072(d)
}

/// Static reference to the MuHash prime.
pub static P: LazyLock<Num3072> = LazyLock::new(p);

/// The constant c = 2^3072 - p = 1103717
const C: u64 = 1103717;

/// A 3072-bit unsigned integer, stored as 48 little-endian u64 limbs.
/// Bit 0 is the LSB of limb 0; bit 3071 is the MSB of limb 47.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Num3072(pub [u64; 48]);

impl Num3072 {
    /// The multiplicative identity: 1.
    pub const ONE: Num3072 = {
        let mut d = [0u64; 48];
        d[0] = 1;
        Num3072(d)
    };

    /// Zero.
    pub const ZERO: Num3072 = Num3072([0u64; 48]);

    /// Returns true if this is zero.
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }

    /// Returns true if this is one.
    pub fn is_one(&self) -> bool {
        self.0[0] == 1 && self.0[1..].iter().all(|&x| x == 0)
    }

    /// Compare two Num3072 values.
    fn cmp_inner(&self, other: &Num3072) -> Ordering {
        for i in (0..48).rev() {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Equal => continue,
                non_eq => return non_eq,
            }
        }
        Ordering::Equal
    }

    /// Subtract other from self, returning the result and borrow flag.
    pub fn sub_with_borrow(&self, other: &Num3072) -> (Num3072, bool) {
        let mut result = [0u64; 48];
        let mut borrow: u64 = 0;
        for i in 0..48 {
            let (a, b) = self.0[i].overflowing_sub(other.0[i]);
            let (c, d) = a.overflowing_sub(borrow);
            result[i] = c;
            borrow = if b || d { 1 } else { 0 };
        }
        (Num3072(result), borrow != 0)
    }

    /// Add other to self, returning the result and carry flag.
    fn add_with_carry(&self, other: &Num3072) -> (Num3072, bool) {
        let mut result = [0u64; 48];
        let mut carry: u64 = 0;
        for i in 0..48 {
            let (a, b) = self.0[i].overflowing_add(other.0[i]);
            let (c, d) = a.overflowing_add(carry);
            result[i] = c;
            carry = if b || d { 1 } else { 0 };
        }
        (Num3072(result), carry != 0)
    }

    /// Modular addition: (self + other) mod P.
    pub fn mod_add(&self, other: &Num3072) -> Num3072 {
        let (sum, carry) = self.add_with_carry(other);
        if carry {
            // sum = self + other - 2^3072 ≡ self + other - c (mod P)
            // But wait: sum = (self + other) - 2^3072, so we need sum + c ≡ self + other (mod P)
            let inc = Num3072::from_u64(C);
            let (r, _) = sum.add_with_carry(&inc);
            // r < P + c, check if >= P
            if r.cmp_inner(&*P) >= Ordering::Equal {
                let (s, _) = r.sub_with_borrow(&*P);
                return s;
            }
            return r;
        }
        if sum.cmp_inner(&*P) >= Ordering::Equal {
            let (s, _) = sum.sub_with_borrow(&*P);
            return s;
        }
        sum
    }

    /// Modular subtraction: (self - other) mod P.
    pub fn mod_sub(&self, other: &Num3072) -> Num3072 {
        let (diff, borrow) = self.sub_with_borrow(other);
        if borrow {
            // diff = self - other + 2^3072
            // We want: diff - 2^3072 + P = self - other (mod P)
            // Since 2^3072 ≡ c (mod P), self - other + 2^3072 ≡ self - other + c (mod P)
            // diff ≡ self - other + c (mod P) is already the right answer!
            // But wait: diff = self - other + 2^3072, and c = 2^3072 - P.
            // So diff = self - other + c + P.
            // diff mod P = (self - other + c) mod P.
            // We need diff - c if we want self - other... no.
            // Actually: we computed diff = self - other + 2^3072 (wrapping).
            // We want self - other mod P.
            // Since 2^3072 ≡ c (mod P):
            // diff ≡ self - other + c (mod P).
            // But we haven't reduced below P yet. diff < 2^3072, c < P.
            // So diff < 2^3072 + c. We need diff mod P.
            // diff ≡ self - other + c ≡ self - other + c (mod P).
            // Hmm, this is getting confusing. Let me think again.
            //
            // self, other, diff all < 2^3072.
            // borrow means self < other, so diff = self + 2^3072 - other.
            // We want: (self - other) mod P = self - other + P.
            // P = 2^3072 - c.
            // self - other + P = self - other + 2^3072 - c = diff - c.
            // If diff >= c: result = diff - c.
            // If diff < c: result = diff - c + P = diff + P - c.
            let c_num = Num3072::from_u64(C);
            if diff.cmp_inner(&c_num) >= Ordering::Equal {
                let (r, _) = diff.sub_with_borrow(&c_num);
                return r;
            } else {
                // diff < c, so diff - c + P = P - (c - diff)
                let (cd, _) = c_num.sub_with_borrow(&diff);
                let (r, _) = (*P).sub_with_borrow(&cd);
                return r;
            }
        }
        diff
    }

    /// Multiply self by a single u64, returning a (3072+64)-bit result as (overflow, low_3072).
    fn mul_u64(&self, rhs: u64) -> (u64, Num3072) {
        let mut result = [0u64; 48];
        let mut carry: u64 = 0;
        for i in 0..48 {
            let wide = self.0[i] as u128 * rhs as u128 + carry as u128;
            result[i] = wide as u64;
            carry = (wide >> 64) as u64;
        }
        (carry, Num3072(result))
    }

    /// Full 3072 × 3072 → 6144 multiplication.
    /// Returns (high_3072_bits, low_3072_bits).
    pub fn mul_full(&self, other: &Num3072) -> (Num3072, Num3072) {
        let mut result = [0u64; 96]; // 96 u64s = 6144 bits
        for i in 0..48 {
            let mut carry: u64 = 0;
            for j in 0..48 {
                let idx = i + j;
                let wide =
                    result[idx] as u128 + self.0[i] as u128 * other.0[j] as u128 + carry as u128;
                result[idx] = wide as u64;
                carry = (wide >> 64) as u64;
            }
            result[i + 48] = result[i + 48].wrapping_add(carry);
        }

        let mut low = [0u64; 48];
        let mut high = [0u64; 48];
        low.copy_from_slice(&result[0..48]);
        high.copy_from_slice(&result[48..96]);
        (Num3072(high), Num3072(low))
    }

    /// Modular multiplication: (self * other) mod P.
    ///
    /// Uses the fast pseudo-Mersenne reduction:
    ///   x = high * 2^3072 + low
    ///   x mod P = low + high * c (mod P) where c = 1103717
    pub fn mod_mul(&self, other: &Num3072) -> Num3072 {
        let (high, low) = self.mul_full(other);

        // Compute high * c, which gives up to 3093 bits
        let (carry_high, prod_low) = high.mul_u64(C);

        // x mod P = low + high * c
        // high * c = carry_high * 2^3072 + prod_low
        let (sum1, carry1) = low.add_with_carry(&prod_low);

        // Remaining: (carry1 + carry_high) * 2^3072 ≡ (carry1 + carry_high) * c (mod P)
        let total_carry = carry1 as u64 + carry_high;
        let carry_num = Num3072::from_u64(total_carry);

        // carry_num * c fits in 64 bits since carry_num ≤ 1103718, c = 1103717
        let (cc_high, cc_low) = carry_num.mul_u64(C);
        debug_assert_eq!(cc_high, 0);

        let (r, c2) = sum1.add_with_carry(&cc_low);
        if c2 {
            // r + 2^3072 ≡ r + c (mod P)
            let inc = Num3072::from_u64(C);
            let (r2, _) = r.add_with_carry(&inc);
            if r2.cmp_inner(&*P) >= Ordering::Equal {
                let (r3, _) = r2.sub_with_borrow(&*P);
                return r3;
            }
            return r2;
        }
        if r.cmp_inner(&*P) >= Ordering::Equal {
            let (r2, _) = r.sub_with_borrow(&*P);
            return r2;
        }
        r
    }

    /// Modular square: self^2 mod P.
    pub fn mod_sqr(&self) -> Num3072 {
        self.mod_mul(self)
    }

    /// Modular inverse: self^(-1) mod P using Fermat's little theorem.
    /// a^(-1) ≡ a^(p-2) mod p. Uses exponentiation by squaring.
    /// Panics if self is 0.
    pub fn mod_inv(&self) -> Num3072 {
        assert!(!self.is_zero(), "modular inverse of 0 is undefined");

        // P - 2: all limbs 0xFF..FF, limb[0] -= 1103718
        let mut p_minus_2 = [0xFFFFFFFFFFFFFFFFu64; 48];
        p_minus_2[0] = 0xFFFFFFFFFFFFFFFFu64 - 1103718u64;
        let exp = Num3072(p_minus_2);

        let mut result = Num3072::ONE;
        let base = *self;
        let mut started = false;

        for limb_idx in (0..48).rev() {
            let limb = exp.0[limb_idx];
            for bit_idx in (0..64).rev() {
                if !started {
                    if limb & (1u64 << bit_idx) != 0 {
                        started = true;
                        result = base;
                    }
                } else {
                    result = result.mod_sqr();
                    if limb & (1u64 << bit_idx) != 0 {
                        result = result.mod_mul(&base);
                    }
                }
            }
        }

        result
    }

    /// Batch modular inversion.
    ///
    /// Returns `values[i]^(-1) mod P` for every input value, using one real
    /// inversion plus O(n) modular multiplications. Multiplicative identities
    /// are skipped so an all-one batch performs no expensive inversion.
    ///
    /// Panics if any input is zero.
    pub fn batch_mod_inv(values: &[Num3072]) -> Vec<Num3072> {
        let mut out = vec![Num3072::ONE; values.len()];
        let mut non_one = Vec::new();

        for (idx, value) in values.iter().copied().enumerate() {
            assert!(!value.is_zero(), "modular inverse of 0 is undefined");
            if !value.is_one() {
                non_one.push((idx, value));
            }
        }

        if non_one.is_empty() {
            return out;
        }

        let mut prefixes = Vec::with_capacity(non_one.len());
        let mut product = Num3072::ONE;
        for (_, value) in &non_one {
            product = product.mod_mul(value);
            prefixes.push(product);
        }

        let mut inverse_product = product.mod_inv();
        for i in (1..non_one.len()).rev() {
            let (idx, value) = non_one[i];
            out[idx] = inverse_product.mod_mul(&prefixes[i - 1]);
            inverse_product = inverse_product.mod_mul(&value);
        }

        out[non_one[0].0] = inverse_product;
        out
    }

    /// Create from a u64.
    pub fn from_u64(x: u64) -> Num3072 {
        let mut d = [0u64; 48];
        d[0] = x;
        Num3072(d)
    }

    /// Serialize to 384 bytes (big-endian).
    pub fn to_bytes(&self) -> [u8; 384] {
        let mut bytes = [0u8; 384];
        for i in 0..48 {
            let limb = self.0[47 - i]; // big-endian: highest limb first
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&limb.to_be_bytes());
        }
        bytes
    }

    /// Serialize to 384 bytes (little-endian), matching Bitcoin Core's Num3072.
    pub fn to_le_bytes(&self) -> [u8; 384] {
        let mut bytes = [0u8; 384];
        for i in 0..48 {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&self.0[i].to_le_bytes());
        }
        bytes
    }

    /// Deserialize from 384 bytes (big-endian).
    pub fn from_bytes(bytes: &[u8; 384]) -> Num3072 {
        let mut d = [0u64; 48];
        for i in 0..48 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            d[47 - i] = u64::from_be_bytes(buf);
        }
        Num3072(d)
    }

    /// Deserialize from 384 bytes (little-endian), matching Bitcoin Core's Num3072.
    pub fn from_le_bytes(bytes: &[u8; 384]) -> Num3072 {
        let mut d = [0u64; 48];
        for i in 0..48 {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            d[i] = u64::from_le_bytes(buf);
        }
        Num3072(d)
    }
}

impl fmt::Debug for Num3072 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Num3072({})", hex::encode(&self.to_bytes()[0..8]))
    }
}

impl fmt::Display for Num3072 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.to_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_mul_basic() {
        let a = Num3072::from_u64(2);
        let b = Num3072::from_u64(3);
        let c = a.mod_mul(&b);
        assert_eq!(c, Num3072::from_u64(6));
    }

    #[test]
    fn test_mod_mul_identity() {
        let a = Num3072::from_u64(42);
        let one = Num3072::ONE;
        assert_eq!(a.mod_mul(&one), a);
    }

    #[test]
    fn test_mod_inv_small() {
        let a = Num3072::from_u64(5);
        let inv = a.mod_inv();
        let prod = a.mod_mul(&inv);
        assert_eq!(prod, Num3072::ONE);
    }

    #[test]
    fn test_mod_inv_large() {
        let mut d = [0u64; 48];
        d[0] = 0xDEADBEEF_CAFEBABE;
        d[1] = 0x12345678_90ABCDEF;
        d[10] = 0xFFFF0000_0000FFFF;
        let a = Num3072(d);
        let inv = a.mod_inv();
        let prod = a.mod_mul(&inv);
        assert_eq!(prod, Num3072::ONE);
    }

    #[test]
    fn test_batch_mod_inv_matches_individual_inverses() {
        let values = vec![
            Num3072::ONE,
            Num3072::from_u64(3),
            Num3072::from_u64(5),
            Num3072::from_u64(42),
        ];

        let inverses = Num3072::batch_mod_inv(&values);
        assert_eq!(inverses.len(), values.len());

        for (value, inverse) in values.iter().zip(inverses) {
            assert_eq!(value.mod_mul(&inverse), Num3072::ONE);
            assert_eq!(inverse, value.mod_inv());
        }
    }

    #[test]
    fn test_mod_add_sub() {
        let a = Num3072::from_u64(100);
        let b = Num3072::from_u64(200);
        let sum = a.mod_add(&b);
        assert_eq!(sum, Num3072::from_u64(300));
        let diff = b.mod_sub(&a);
        assert_eq!(diff, Num3072::from_u64(100));
    }

    #[test]
    fn test_mod_sub_wraparound() {
        let a = Num3072::from_u64(5);
        let b = Num3072::from_u64(10);
        let diff = a.mod_sub(&b); // 5 - 10 mod P = P - 5
        let (expected, _) = (*P).sub_with_borrow(&Num3072::from_u64(5));
        assert_eq!(diff, expected);
    }

    #[test]
    fn test_mul_mod_p_boundary() {
        // (P - 1) * (P - 1) mod P = 1
        let (pm1, _) = (*P).sub_with_borrow(&Num3072::ONE);
        let result = pm1.mod_mul(&pm1);
        assert_eq!(result, Num3072::ONE);

        // (P - 1) * 1 mod P = P - 1
        let result2 = pm1.mod_mul(&Num3072::ONE);
        assert_eq!(result2, pm1);
    }

    #[test]
    fn test_mul_u64() {
        // Multiply a 3072-bit number (only lowest limb set) by 2
        // The carry propagates through all zero limbs and there's no overflow past 48 limbs
        let a = Num3072::from_u64(0xFFFFFFFFFFFFFFFF);
        let (carry, low) = a.mul_u64(2);
        assert_eq!(low.0[0], 0xFFFFFFFFFFFFFFFE);
        assert_eq!(low.0[1], 1); // carry propagated into next limb
        assert_eq!(carry, 0); // no overflow past the 48th limb
    }

    #[test]
    fn test_bytes_roundtrip() {
        let mut d = [0u64; 48];
        for i in 0..48 {
            d[i] = (i as u64).wrapping_mul(0x1111111111111111u64);
        }
        let a = Num3072(d);
        let bytes = a.to_bytes();
        let b = Num3072::from_bytes(&bytes);
        assert_eq!(a, b);
    }
}
