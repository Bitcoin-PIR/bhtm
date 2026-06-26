//! Slim MuHash builder over externally prepared stripped block deltas.
//!
//! The builder verifies the light-client-style header binding: supplied
//! stripped transaction commitments match the supplied header, the header hashes
//! to the trusted block hash, and each new header links to the previous trusted
//! or processed block hash. It does not validate proof-of-work, scripts,
//! coinbase rules, BIP30, duplicate txids, or UTXO membership. The data source
//! is responsible for preparing Bitcoin Core-compatible coin additions/removals.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use std::time::Instant;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::merkle_tree::{
    inclusion_proof, verify_inclusion_proof, MerkleProofStep, MuHashMerkleTree,
};
use crate::muhash::{MuHash, MuHashParts, UtxoEntry};

const MAINNET_GENESIS_HASH: [u8; 32] = [
    0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63, 0xf7, 0x4f,
    0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const BINARY_MAGIC: &[u8; 8] = b"BHTMBIN1";
const BINARY_ANCHOR_BEFORE_GENESIS: u8 = 0;
const BINARY_ANCHOR_TRUSTED_BLOCK: u8 = 1;
const MUHASH_NORMALIZATION_BATCH_SIZE: usize = 1024;

/// One stripped block plus the MuHash coin delta prepared by the data source.
#[derive(Debug, Clone)]
pub struct StrippedBlockDelta {
    /// Block height. The builder requires this to advance one at a time.
    pub height: u32,
    /// Bitcoin block header serialization.
    pub header: [u8; 80],
    /// Trusted block hash the header must match, in Bitcoin internal byte order.
    pub block_hash: [u8; 32],
    /// Non-witness transaction ids committed by the header Merkle root.
    pub txids: Vec<[u8; 32]>,
    /// Coins to add to MuHash. Must already follow Bitcoin Core coinstats rules.
    pub additions: Vec<UtxoEntry>,
    /// Coins to remove from MuHash. Must include the full spent coin data.
    pub removals: Vec<UtxoEntry>,
}

/// Trusted starting point for a MuHash builder run.
#[derive(Debug, Clone)]
pub struct TrustedAnchor {
    /// Height of the trusted block anchor, or 0 for the before-genesis anchor.
    pub start_height: u32,
    /// Trusted block hash at `start_height`, or zero for the before-genesis anchor.
    pub start_block_hash: [u8; 32],
    /// Trusted normalized MuHash accumulator value at `start_height`.
    pub start_muhash_value: [u8; 384],
    next_height: u32,
}

impl TrustedAnchor {
    /// Start before genesis. The first processed block must be height 0 and
    /// must have an all-zero previous block hash.
    pub fn before_genesis() -> Self {
        TrustedAnchor {
            start_height: 0,
            start_block_hash: [0u8; 32],
            start_muhash_value: MuHash::new().to_bytes(),
            next_height: 0,
        }
    }

    /// Start from a trusted block. The first processed block must be
    /// `height + 1` and link to `block_hash`.
    pub fn from_block(height: u32, block_hash: [u8; 32], muhash_value: [u8; 384]) -> Self {
        TrustedAnchor {
            start_height: height,
            start_block_hash: block_hash,
            start_muhash_value: muhash_value,
            next_height: height.saturating_add(1),
        }
    }

    /// Start from the Bitcoin mainnet genesis block.
    ///
    /// Bitcoin Core coinstats excludes genesis outputs, so the trusted MuHash
    /// value at this anchor is the empty accumulator.
    pub fn mainnet_genesis() -> Self {
        Self::from_block(0, MAINNET_GENESIS_HASH, MuHash::new().to_bytes())
    }

    /// Full normalized trusted starting MuHash value.
    pub fn start_muhash(&self) -> [u8; 384] {
        self.start_muhash_value
    }
}

impl StrippedBlockDelta {
    /// Verify the stripped block against its header and trusted header hash.
    pub fn verify_header_binding(&self) -> io::Result<()> {
        let computed_hash = block_hash_from_header(&self.header);
        if computed_hash != self.block_hash {
            return invalid_data(format!(
                "header hash mismatch at height {}: expected {}, got {}",
                self.height,
                hex::encode(self.block_hash),
                hex::encode(computed_hash)
            ));
        }

        let computed_merkle_root = merkle_root_from_txids(&self.txids)?;
        let header_merkle_root = header_merkle_root(&self.header);
        if computed_merkle_root != header_merkle_root {
            return invalid_data(format!(
                "txid merkle root mismatch at height {}: expected {}, got {}",
                self.height,
                hex::encode(header_merkle_root),
                hex::encode(computed_merkle_root)
            ));
        }

        Ok(())
    }
}

/// Statistics from a slim builder run.
#[derive(Debug, Clone)]
pub struct ProcessStats {
    /// Trusted starting height.
    pub start_height: u32,
    /// Trusted starting block hash, in Bitcoin internal byte order.
    pub start_block_hash: [u8; 32],
    /// Trusted normalized starting MuHash value.
    pub start_muhash: [u8; 384],
    /// Number of stripped block deltas processed.
    pub blocks_processed: u64,
    /// Number of txids checked against block Merkle roots.
    pub txids_checked: u64,
    /// Number of coin additions applied to MuHash.
    pub coins_added: u64,
    /// Number of coin removals applied to MuHash.
    pub coins_removed: u64,
    /// Last processed height.
    pub end_height: u32,
    /// Last processed block hash, in Bitcoin internal byte order.
    pub end_block_hash: [u8; 32],
    /// Normalized ending MuHash value.
    pub end_muhash: [u8; 384],
    /// MuHash binding tree size.
    pub tree_size: u64,
    /// MuHash binding tree root.
    pub tree_root: [u8; 32],
    /// Time elapsed.
    pub elapsed_secs: f64,
    /// Throughput in blocks/second.
    pub blocks_per_sec: f64,
    /// Throughput in checked txids/second.
    pub txids_per_sec: f64,
}

/// One block/MuHash leaf committed by a chunk tree.
#[derive(Debug, Clone)]
pub struct MuHashLeafRecord {
    pub leaf_index: u64,
    pub height: u32,
    pub block_hash: [u8; 32],
    pub muhash: [u8; 384],
    pub leaf_hash: [u8; 32],
    pub core_muhash: [u8; 32],
}

/// Inclusion proof for one block/MuHash leaf plus full chunk stats.
#[derive(Debug, Clone)]
pub struct MuHashLeafProof {
    pub stats: ProcessStats,
    pub leaf: MuHashLeafRecord,
    pub proof: Vec<MerkleProofStep>,
}

/// Runtime job binding for attestation v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobAttestationContext {
    /// Version of the canonical `job.json` schema.
    pub job_version: u16,
    /// SHA256 of the canonical `job.json` bytes.
    pub job_sha256: [u8; 32],
}

/// Minimal state owned by the MuHash builder.
pub struct SlimMuHashBuilder {
    anchor: TrustedAnchor,
    muhash: MuHash,
    merkle_tree: MuHashMerkleTree,
    blocks_processed: u64,
    txids_checked: u64,
    coins_added: u64,
    coins_removed: u64,
    next_height: u32,
    previous_block_hash: [u8; 32],
}

struct PendingMuHashLeaf {
    height: u32,
    block_hash: [u8; 32],
    muhash_parts: MuHashParts,
}

impl SlimMuHashBuilder {
    /// Create a new builder starting before genesis.
    pub fn new() -> Self {
        Self::from_anchor(TrustedAnchor::before_genesis())
    }

    /// Create a new builder from a trusted start anchor.
    pub fn from_anchor(anchor: TrustedAnchor) -> Self {
        SlimMuHashBuilder {
            muhash: MuHash::from_normalized_bytes(&anchor.start_muhash_value),
            merkle_tree: MuHashMerkleTree::new(),
            blocks_processed: 0,
            txids_checked: 0,
            coins_added: 0,
            coins_removed: 0,
            next_height: anchor.next_height,
            previous_block_hash: anchor.start_block_hash,
            anchor,
        }
    }

    /// Verify one stripped block delta and apply its MuHash delta.
    pub fn process_block(&mut self, block: &StrippedBlockDelta) -> io::Result<()> {
        let pending = self.process_block_deferred_leaf(block)?;
        let muhash = MuHash::normalized_bytes_from_parts(pending.muhash_parts);
        self.merkle_tree
            .append_block(pending.height, &pending.block_hash, &muhash);

        Ok(())
    }

    fn process_block_deferred_leaf(
        &mut self,
        block: &StrippedBlockDelta,
    ) -> io::Result<PendingMuHashLeaf> {
        if block.height != self.next_height {
            return invalid_data(format!(
                "height mismatch: expected {}, got {}",
                self.next_height, block.height
            ));
        }

        let prev_hash = prev_block_hash_from_header(&block.header);
        if prev_hash != self.previous_block_hash {
            return invalid_data(format!(
                "prev_block_hash mismatch at height {}: expected {}, got {}",
                block.height,
                hex::encode(self.previous_block_hash),
                hex::encode(prev_hash)
            ));
        }

        block.verify_header_binding()?;

        for coin in &block.additions {
            self.muhash.add(coin);
        }
        for coin in &block.removals {
            self.muhash.remove(coin);
        }

        let pending_leaf = PendingMuHashLeaf {
            height: block.height,
            block_hash: block.block_hash,
            muhash_parts: self.muhash.parts(),
        };

        self.blocks_processed += 1;
        self.txids_checked += block.txids.len() as u64;
        self.coins_added += block.additions.len() as u64;
        self.coins_removed += block.removals.len() as u64;
        self.previous_block_hash = block.block_hash;
        self.next_height = block
            .height
            .checked_add(1)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "block height overflow"))?;

        Ok(pending_leaf)
    }

    /// Finalize run statistics.
    pub fn stats(&self, start_time: Instant) -> ProcessStats {
        let elapsed = start_time.elapsed().as_secs_f64();
        ProcessStats {
            start_height: self.anchor.start_height,
            start_block_hash: self.anchor.start_block_hash,
            start_muhash: self.anchor.start_muhash(),
            blocks_processed: self.blocks_processed,
            txids_checked: self.txids_checked,
            coins_added: self.coins_added,
            coins_removed: self.coins_removed,
            end_height: self.next_height.saturating_sub(1),
            end_block_hash: self.previous_block_hash,
            end_muhash: self.muhash.to_bytes(),
            tree_size: self.merkle_tree.size(),
            tree_root: *self.merkle_tree.root(),
            elapsed_secs: elapsed,
            blocks_per_sec: self.blocks_processed as f64 / elapsed.max(0.001),
            txids_per_sec: self.txids_checked as f64 / elapsed.max(0.001),
        }
    }
}

impl Default for SlimMuHashBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Process a sequence of stripped block deltas.
pub fn process_stripped_blocks<I>(blocks: I) -> io::Result<ProcessStats>
where
    I: IntoIterator<Item = StrippedBlockDelta>,
{
    process_stripped_blocks_from_anchor(TrustedAnchor::before_genesis(), blocks)
}

/// Process a sequence of stripped block deltas from a trusted start anchor.
pub fn process_stripped_blocks_from_anchor<I>(
    anchor: TrustedAnchor,
    blocks: I,
) -> io::Result<ProcessStats>
where
    I: IntoIterator<Item = StrippedBlockDelta>,
{
    process_stripped_block_results_from_anchor(anchor, blocks.into_iter().map(Ok))
}

/// Process stripped block deltas from a JSONL file.
pub fn process_jsonl_file(path: &Path) -> io::Result<ProcessStats> {
    let file = File::open(path)?;
    process_jsonl_reader(BufReader::new(file))
}

/// Process stripped block deltas from a JSONL reader.
pub fn process_jsonl_reader<R: BufRead>(reader: R) -> io::Result<ProcessStats> {
    let blocks = reader
        .lines()
        .enumerate()
        .filter_map(|(line_no, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            Ok(line) => Some(parse_json_block_line(&line, line_no + 1)),
            Err(e) => Some(Err(e)),
        });

    process_stripped_block_results_from_anchor(TrustedAnchor::before_genesis(), blocks)
}

/// Process compact binary stripped block deltas from a file.
pub fn process_binary_file(path: &Path) -> io::Result<ProcessStats> {
    let file = File::open(path)?;
    process_binary_reader(BufReader::new(file))
}

/// Process compact binary stripped block deltas from a reader.
pub fn process_binary_reader<R: Read>(mut reader: R) -> io::Result<ProcessStats> {
    let anchor = read_binary_header(&mut reader)?;
    let blocks = BinaryBlockIter { reader };
    process_stripped_block_results_from_anchor(anchor, blocks)
}

/// Generate a block/MuHash leaf inclusion proof from a compact binary stream.
pub fn prove_binary_leaf_file(path: &Path, target_height: u32) -> io::Result<MuHashLeafProof> {
    let file = File::open(path)?;
    prove_binary_leaf_reader(BufReader::new(file), target_height)
}

/// Generate a block/MuHash leaf inclusion proof from a compact binary reader.
pub fn prove_binary_leaf_reader<R: Read>(
    mut reader: R,
    target_height: u32,
) -> io::Result<MuHashLeafProof> {
    let anchor = read_binary_header(&mut reader)?;
    let blocks = BinaryBlockIter { reader };
    prove_stripped_block_results_from_anchor(anchor, blocks, target_height)
}

fn process_stripped_block_results_from_anchor<I>(
    anchor: TrustedAnchor,
    blocks: I,
) -> io::Result<ProcessStats>
where
    I: IntoIterator<Item = io::Result<StrippedBlockDelta>>,
{
    let start_time = Instant::now();
    let mut builder = SlimMuHashBuilder::from_anchor(anchor);
    let mut pending_leaves = Vec::with_capacity(MUHASH_NORMALIZATION_BATCH_SIZE);

    for block in blocks {
        let pending_leaf = builder.process_block_deferred_leaf(&block?)?;
        pending_leaves.push(pending_leaf);
        if pending_leaves.len() >= MUHASH_NORMALIZATION_BATCH_SIZE {
            flush_pending_muhash_leaves(&mut builder.merkle_tree, &mut pending_leaves);
        }
        report_progress(&builder, start_time, 1000);
    }

    flush_pending_muhash_leaves(&mut builder.merkle_tree, &mut pending_leaves);

    Ok(builder.stats(start_time))
}

fn prove_stripped_block_results_from_anchor<I>(
    anchor: TrustedAnchor,
    blocks: I,
    target_height: u32,
) -> io::Result<MuHashLeafProof>
where
    I: IntoIterator<Item = io::Result<StrippedBlockDelta>>,
{
    let start_time = Instant::now();
    let mut builder = SlimMuHashBuilder::from_anchor(anchor);
    let mut pending_leaves = Vec::with_capacity(MUHASH_NORMALIZATION_BATCH_SIZE);
    let mut leaf_hashes = Vec::<[u8; 32]>::new();
    let mut target_leaf = None::<MuHashLeafRecord>;

    for block in blocks {
        let pending_leaf = builder.process_block_deferred_leaf(&block?)?;
        pending_leaves.push(pending_leaf);
        if pending_leaves.len() >= MUHASH_NORMALIZATION_BATCH_SIZE {
            flush_pending_muhash_leaves_with_observer(
                &mut builder.merkle_tree,
                &mut pending_leaves,
                &mut |leaf, muhash, leaf_hash| {
                    let leaf_index = leaf_hashes.len() as u64;
                    if leaf.height == target_height {
                        target_leaf = Some(MuHashLeafRecord {
                            leaf_index,
                            height: leaf.height,
                            block_hash: leaf.block_hash,
                            muhash: *muhash,
                            leaf_hash,
                            core_muhash: core_muhash_digest(muhash),
                        });
                    }
                    leaf_hashes.push(leaf_hash);
                },
            );
        }
        report_progress(&builder, start_time, 1000);
    }

    flush_pending_muhash_leaves_with_observer(
        &mut builder.merkle_tree,
        &mut pending_leaves,
        &mut |leaf, muhash, leaf_hash| {
            let leaf_index = leaf_hashes.len() as u64;
            if leaf.height == target_height {
                target_leaf = Some(MuHashLeafRecord {
                    leaf_index,
                    height: leaf.height,
                    block_hash: leaf.block_hash,
                    muhash: *muhash,
                    leaf_hash,
                    core_muhash: core_muhash_digest(muhash),
                });
            }
            leaf_hashes.push(leaf_hash);
        },
    );

    let stats = builder.stats(start_time);
    let leaf = target_leaf.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "target height {} not found in processed range {}..={}",
                target_height,
                stats.start_height.saturating_add(1),
                stats.end_height
            ),
        )
    })?;
    let proof = inclusion_proof(&leaf_hashes, leaf.leaf_index as usize)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    verify_inclusion_proof(
        leaf.leaf_hash,
        leaf.leaf_index as usize,
        leaf_hashes.len(),
        &proof,
        stats.tree_root,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(MuHashLeafProof { stats, leaf, proof })
}

fn flush_pending_muhash_leaves(
    merkle_tree: &mut MuHashMerkleTree,
    pending_leaves: &mut Vec<PendingMuHashLeaf>,
) {
    flush_pending_muhash_leaves_with_observer(merkle_tree, pending_leaves, &mut |_, _, _| {});
}

fn flush_pending_muhash_leaves_with_observer<F>(
    merkle_tree: &mut MuHashMerkleTree,
    pending_leaves: &mut Vec<PendingMuHashLeaf>,
    observer: &mut F,
) where
    F: FnMut(&PendingMuHashLeaf, &[u8; 384], [u8; 32]),
{
    if pending_leaves.is_empty() {
        return;
    }

    let parts = pending_leaves
        .iter()
        .map(|leaf| leaf.muhash_parts)
        .collect::<Vec<_>>();
    let muhash_values = MuHash::normalized_bytes_batch(&parts);

    for (leaf, muhash) in pending_leaves.iter().zip(muhash_values) {
        let leaf_hash = MuHashMerkleTree::leaf_hash(leaf.height, &leaf.block_hash, &muhash);
        merkle_tree.append(leaf_hash);
        observer(leaf, &muhash, leaf_hash);
    }

    pending_leaves.clear();
}

pub fn core_muhash_digest(muhash: &[u8; 384]) -> [u8; 32] {
    let digest = Sha256::digest(muhash);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Compute Bitcoin's block hash from an 80-byte header.
pub fn block_hash_from_header(header: &[u8; 80]) -> [u8; 32] {
    sha256d(header)
}

/// Return the Merkle root committed in an 80-byte Bitcoin block header.
pub fn header_merkle_root(header: &[u8; 80]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&header[36..68]);
    out
}

/// Return the previous block hash committed in an 80-byte Bitcoin block header.
pub fn prev_block_hash_from_header(header: &[u8; 80]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&header[4..36]);
    out
}

/// Compute the Bitcoin txid Merkle root from internal-order txid bytes.
pub fn merkle_root_from_txids(txids: &[[u8; 32]]) -> io::Result<[u8; 32]> {
    if txids.is_empty() {
        return invalid_data("stripped block has no txids");
    }

    let mut layer = txids.to_vec();
    while layer.len() > 1 {
        let mut next = Vec::with_capacity((layer.len() + 1) / 2);
        for pair in layer.chunks(2) {
            let left = pair[0];
            let right = if pair.len() == 2 { pair[1] } else { pair[0] };
            let mut data = [0u8; 64];
            data[..32].copy_from_slice(&left);
            data[32..].copy_from_slice(&right);
            next.push(sha256d(&data));
        }
        layer = next;
    }

    Ok(layer[0])
}

/// Output an attestation payload. SEV signing is handled by the caller.
///
/// Each chunk is proven **independently** of every other chunk — the
/// TEE binds only this chunk's `(start_height, end_height, start_/
/// end_block_hash, tree_root, tree_size, start/end_muhash)`. Chain
/// continuity is checked *outside* the TEE by comparing one chunk's
/// `start_block_hash` + `start_muhash` against the canonical Bitcoin
/// chain and the prior chunk's `end_block_hash` + `end_muhash`.
/// Bitcoin consensus makes the block-hash chain unforgeable, so a
/// spliced chunk would have a `start_block_hash` that doesn't link to
/// the prior chunk's `end_block_hash`, and the per-block chain-linkage
/// check the TEE runs would have rejected it during processing.
///
/// This makes every chunk's attestation valid in isolation: a client
/// who knows a single (height, block_hash, muhash) triple can verify
/// any chunk that covers that height by checking the Merkle leaf
/// against `tree_root`, without needing any sibling chunk's
/// attestation.
pub fn produce_attestation(stats: &ProcessStats) -> Vec<u8> {
    let mut attestation = Vec::new();

    attestation.extend_from_slice(&1u16.to_le_bytes());
    attestation.extend_from_slice(&stats.start_height.to_le_bytes());
    attestation.extend_from_slice(&stats.end_height.to_le_bytes());
    attestation.extend_from_slice(&stats.start_block_hash);
    attestation.extend_from_slice(&stats.end_block_hash);
    attestation.extend_from_slice(&[0u8; 32]); // reserved: was prev_tree_root in v2; always zero in v1
    attestation.extend_from_slice(&stats.tree_root);
    attestation.extend_from_slice(&stats.tree_size.to_le_bytes());
    attestation.extend_from_slice(&stats.start_muhash);
    attestation.extend_from_slice(&stats.end_muhash);

    attestation
}

/// Output a job-bound attestation payload.
///
/// Version 2 keeps the v1 chunk-result fields intact, but makes the
/// formerly-reserved 32-byte slot carry `job_sha256` and inserts a
/// 2-byte `job_version` after the attestation version. This is the
/// measured-generic-UKI path: chunk-specific inputs live in `job.json`,
/// and the TEE quote binds the exact canonical job by hash.
pub fn produce_job_attestation(stats: &ProcessStats, context: JobAttestationContext) -> Vec<u8> {
    let mut attestation = Vec::new();

    attestation.extend_from_slice(&2u16.to_le_bytes());
    attestation.extend_from_slice(&context.job_version.to_le_bytes());
    attestation.extend_from_slice(&stats.start_height.to_le_bytes());
    attestation.extend_from_slice(&stats.end_height.to_le_bytes());
    attestation.extend_from_slice(&stats.start_block_hash);
    attestation.extend_from_slice(&stats.end_block_hash);
    attestation.extend_from_slice(&context.job_sha256);
    attestation.extend_from_slice(&stats.tree_root);
    attestation.extend_from_slice(&stats.tree_size.to_le_bytes());
    attestation.extend_from_slice(&stats.start_muhash);
    attestation.extend_from_slice(&stats.end_muhash);

    attestation
}

fn sha256d(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}

struct BinaryBlockIter<R> {
    reader: R,
}

impl<R: Read> Iterator for BinaryBlockIter<R> {
    type Item = io::Result<StrippedBlockDelta>;

    fn next(&mut self) -> Option<Self::Item> {
        match read_binary_block(&mut self.reader) {
            Ok(Some(block)) => Some(Ok(block)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

fn read_binary_header<R: Read>(reader: &mut R) -> io::Result<TrustedAnchor> {
    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != BINARY_MAGIC {
        return invalid_data(format!("invalid binary stream magic: {:02x?}", magic));
    }

    let anchor_kind = read_u8(reader)?;
    let start_height = read_u32(reader)?;
    let start_block_hash = read_array(reader)?;
    let start_muhash = read_array(reader)?;

    match anchor_kind {
        BINARY_ANCHOR_BEFORE_GENESIS => {
            if start_height != 0 || start_block_hash != [0u8; 32] {
                return invalid_data("before-genesis anchor must use height 0 and zero block hash");
            }
            Ok(TrustedAnchor::before_genesis())
        }
        BINARY_ANCHOR_TRUSTED_BLOCK => Ok(TrustedAnchor::from_block(
            start_height,
            start_block_hash,
            start_muhash,
        )),
        other => invalid_data(format!("unknown binary anchor kind {}", other)),
    }
}

fn read_binary_block<R: Read>(reader: &mut R) -> io::Result<Option<StrippedBlockDelta>> {
    let first = match read_optional_first_byte(reader)? {
        Some(byte) => byte,
        None => return Ok(None),
    };

    let height = read_u32_after_first(reader, first)?;
    let header = read_array(reader)?;
    let block_hash = read_array(reader)?;

    let tx_count = read_u32(reader)?;
    let mut txids = Vec::with_capacity(tx_count as usize);
    for _ in 0..tx_count {
        let tx = read_len_prefixed_bytes(reader)?;
        txids.push(sha256d(&tx));
    }

    let add_count = read_u32(reader)?;
    let mut additions = Vec::with_capacity(add_count as usize);
    for _ in 0..add_count {
        additions.push(read_binary_coin(reader)?);
    }

    let remove_count = read_u32(reader)?;
    let mut removals = Vec::with_capacity(remove_count as usize);
    for _ in 0..remove_count {
        removals.push(read_binary_coin(reader)?);
    }

    Ok(Some(StrippedBlockDelta {
        height,
        header,
        block_hash,
        txids,
        additions,
        removals,
    }))
}

fn read_binary_coin<R: Read>(reader: &mut R) -> io::Result<UtxoEntry> {
    let txid = read_array(reader)?;
    let vout = read_u32(reader)?;
    let amount = read_u64(reader)?;
    let height = read_u32(reader)?;
    let is_coinbase = match read_u8(reader)? {
        0 => false,
        1 => true,
        other => return invalid_data(format!("invalid coinbase flag {}", other)),
    };
    let script_pubkey = read_len_prefixed_bytes(reader)?;

    Ok(UtxoEntry {
        txid,
        vout,
        amount,
        script_pubkey,
        height,
        is_coinbase,
    })
}

fn read_optional_first_byte<R: Read>(reader: &mut R) -> io::Result<Option<u8>> {
    let mut byte = [0u8; 1];
    match reader.read_exact(&mut byte) {
        Ok(()) => Ok(Some(byte[0])),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(e),
    }
}

fn read_u8<R: Read>(reader: &mut R) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32<R: Read>(reader: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u32_after_first<R: Read>(reader: &mut R, first: u8) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    buf[0] = first;
    reader.read_exact(&mut buf[1..])?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64<R: Read>(reader: &mut R) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_array<R: Read, const N: usize>(reader: &mut R) -> io::Result<[u8; N]> {
    let mut out = [0u8; N];
    reader.read_exact(&mut out)?;
    Ok(out)
}

fn read_len_prefixed_bytes<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
    let len = read_u32(reader)? as usize;
    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn report_progress(builder: &SlimMuHashBuilder, start_time: Instant, interval: u64) {
    if builder.blocks_processed % interval == 0 || builder.blocks_processed == 1 {
        let elapsed = start_time.elapsed().as_secs_f64();
        eprintln!(
            "Height {} | {} blocks | {} txids | +{} -{} coins | {:.1} blk/s",
            builder.next_height.saturating_sub(1),
            builder.blocks_processed,
            builder.txids_checked,
            builder.coins_added,
            builder.coins_removed,
            builder.blocks_processed as f64 / elapsed.max(0.001),
        );
    }
}

#[derive(Debug, Deserialize)]
struct JsonStrippedBlockDelta {
    height: u32,
    header: String,
    block_hash: String,
    #[serde(default)]
    txids: Option<Vec<String>>,
    #[serde(default, alias = "stripped_transactions")]
    stripped_txs: Vec<String>,
    #[serde(default, alias = "adds", alias = "added")]
    additions: Vec<JsonUtxoEntry>,
    #[serde(default, alias = "removes", alias = "removed")]
    removals: Vec<JsonUtxoEntry>,
}

#[derive(Debug, Deserialize)]
struct JsonUtxoEntry {
    txid: String,
    vout: u32,
    amount: u64,
    script_pubkey: String,
    height: u32,
    is_coinbase: bool,
}

fn parse_json_block_line(line: &str, line_no: usize) -> io::Result<StrippedBlockDelta> {
    let parsed: JsonStrippedBlockDelta = serde_json::from_str(line).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSONL block at line {}: {}", line_no, e),
        )
    })?;

    parsed.into_delta(line_no)
}

impl JsonStrippedBlockDelta {
    fn into_delta(self, line_no: usize) -> io::Result<StrippedBlockDelta> {
        let txids = self.decode_txids(line_no)?;
        Ok(StrippedBlockDelta {
            height: self.height,
            header: hex_to_array(&self.header, line_no, "header")?,
            block_hash: hex_to_array(&self.block_hash, line_no, "block_hash")?,
            txids,
            additions: self
                .additions
                .into_iter()
                .enumerate()
                .map(|(idx, coin)| coin.into_utxo(line_no, &format!("additions[{}]", idx)))
                .collect::<io::Result<Vec<_>>>()?,
            removals: self
                .removals
                .into_iter()
                .enumerate()
                .map(|(idx, coin)| coin.into_utxo(line_no, &format!("removals[{}]", idx)))
                .collect::<io::Result<Vec<_>>>()?,
        })
    }

    fn decode_txids(&self, line_no: usize) -> io::Result<Vec<[u8; 32]>> {
        let provided = self
            .txids
            .as_ref()
            .map(|txids| {
                txids
                    .iter()
                    .enumerate()
                    .map(|(idx, txid)| hex_to_array(txid, line_no, &format!("txids[{}]", idx)))
                    .collect::<io::Result<Vec<_>>>()
            })
            .transpose()?;

        let derived = self
            .stripped_txs
            .iter()
            .enumerate()
            .map(|(idx, tx)| {
                let tx_bytes = hex::decode(tx).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "invalid hex at line {} field stripped_txs[{}]: {}",
                            line_no, idx, e
                        ),
                    )
                })?;
                Ok(sha256d(&tx_bytes))
            })
            .collect::<io::Result<Vec<_>>>()?;

        match (provided, derived.is_empty()) {
            (None, true) => invalid_data(format!(
                "line {} must provide txids or stripped_txs",
                line_no
            )),
            (None, false) => Ok(derived),
            (Some(txids), true) => Ok(txids),
            (Some(txids), false) => {
                if txids != derived {
                    return invalid_data(format!(
                        "line {} txids do not match SHA256d(stripped_txs)",
                        line_no
                    ));
                }
                Ok(txids)
            }
        }
    }
}

impl JsonUtxoEntry {
    fn into_utxo(self, line_no: usize, field: &str) -> io::Result<UtxoEntry> {
        Ok(UtxoEntry {
            txid: hex_to_array(&self.txid, line_no, &format!("{}.txid", field))?,
            vout: self.vout,
            amount: self.amount,
            script_pubkey: hex::decode(&self.script_pubkey).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "invalid hex at line {} field {}.script_pubkey: {}",
                        line_no, field, e
                    ),
                )
            })?,
            height: self.height,
            is_coinbase: self.is_coinbase,
        })
    }
}

fn hex_to_array<const N: usize>(s: &str, line_no: usize, field: &str) -> io::Result<[u8; N]> {
    let bytes = hex::decode(s).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid hex at line {} field {}: {}", line_no, field, e),
        )
    })?;
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "invalid byte length at line {} field {}: expected {}, got {}",
                line_no,
                field,
                N,
                bytes.len()
            ),
        )
    })
}

fn invalid_data<T>(message: impl Into<String>) -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::InvalidData, message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_with_prev_merkle(prev_hash: [u8; 32], merkle_root: [u8; 32], nonce: u32) -> [u8; 80] {
        let mut header = [0u8; 80];
        header[..4].copy_from_slice(&1u32.to_le_bytes());
        header[4..36].copy_from_slice(&prev_hash);
        header[36..68].copy_from_slice(&merkle_root);
        header[68..72].copy_from_slice(&1231006505u32.to_le_bytes());
        header[72..76].copy_from_slice(&0x1d00ffffu32.to_le_bytes());
        header[76..80].copy_from_slice(&nonce.to_le_bytes());
        header
    }

    fn coin(tag: u8) -> UtxoEntry {
        UtxoEntry {
            txid: [tag; 32],
            vout: 0,
            amount: 1000,
            script_pubkey: vec![0x51],
            height: tag as u32,
            is_coinbase: false,
        }
    }

    fn binary_header_before_genesis(out: &mut Vec<u8>) {
        out.extend_from_slice(BINARY_MAGIC);
        out.push(BINARY_ANCHOR_BEFORE_GENESIS);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&[0u8; 32]);
        out.extend_from_slice(&MuHash::new().to_bytes());
    }

    fn write_binary_block(out: &mut Vec<u8>, block: &StrippedBlockDelta, stripped_txs: &[Vec<u8>]) {
        out.extend_from_slice(&block.height.to_le_bytes());
        out.extend_from_slice(&block.header);
        out.extend_from_slice(&block.block_hash);
        out.extend_from_slice(&(stripped_txs.len() as u32).to_le_bytes());
        for tx in stripped_txs {
            out.extend_from_slice(&(tx.len() as u32).to_le_bytes());
            out.extend_from_slice(tx);
        }
        out.extend_from_slice(&(block.additions.len() as u32).to_le_bytes());
        for coin in &block.additions {
            write_binary_coin(out, coin);
        }
        out.extend_from_slice(&(block.removals.len() as u32).to_le_bytes());
        for coin in &block.removals {
            write_binary_coin(out, coin);
        }
    }

    fn write_binary_coin(out: &mut Vec<u8>, coin: &UtxoEntry) {
        out.extend_from_slice(&coin.txid);
        out.extend_from_slice(&coin.vout.to_le_bytes());
        out.extend_from_slice(&coin.amount.to_le_bytes());
        out.extend_from_slice(&coin.height.to_le_bytes());
        out.push(u8::from(coin.is_coinbase));
        out.extend_from_slice(&(coin.script_pubkey.len() as u32).to_le_bytes());
        out.extend_from_slice(&coin.script_pubkey);
    }

    fn delta(height: u32, nonce: u32, txids: Vec<[u8; 32]>) -> StrippedBlockDelta {
        linked_delta(height, [0u8; 32], nonce, txids)
    }

    fn linked_delta(
        height: u32,
        prev_hash: [u8; 32],
        nonce: u32,
        txids: Vec<[u8; 32]>,
    ) -> StrippedBlockDelta {
        let merkle_root = merkle_root_from_txids(&txids).unwrap();
        let header = header_with_prev_merkle(prev_hash, merkle_root, nonce);
        StrippedBlockDelta {
            height,
            header,
            block_hash: block_hash_from_header(&header),
            txids,
            additions: Vec::new(),
            removals: Vec::new(),
        }
    }

    #[test]
    fn test_header_binding_valid_single_txid() {
        let block = delta(7, 1, vec![[0x11; 32]]);
        block.verify_header_binding().unwrap();
    }

    #[test]
    fn test_header_binding_rejects_bad_hash() {
        let mut block = delta(7, 1, vec![[0x11; 32]]);
        block.block_hash[0] ^= 1;

        let err = block.verify_header_binding().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("header hash mismatch"));
    }

    #[test]
    fn test_header_binding_rejects_bad_merkle_root() {
        let mut block = delta(7, 1, vec![[0x11; 32]]);
        block.txids[0] = [0x22; 32];

        let err = block.verify_header_binding().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("txid merkle root mismatch"));
    }

    #[test]
    fn test_merkle_root_duplicates_odd_leaf() {
        let txids = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let root = merkle_root_from_txids(&txids).unwrap();

        let left = sha256d(&[[1u8; 32], [2u8; 32]].concat());
        let right = sha256d(&[[3u8; 32], [3u8; 32]].concat());
        let expected = sha256d(&[left, right].concat());
        assert_eq!(root, expected);
    }

    #[test]
    fn test_builder_applies_coin_deltas_without_utxo_state() {
        let entry = coin(5);
        let mut first = delta(0, 1, vec![[0x11; 32]]);
        first.additions.push(entry.clone());
        let mut second = linked_delta(1, first.block_hash, 2, vec![[0x22; 32]]);
        second.removals.push(entry);

        let stats = process_stripped_blocks(vec![first, second]).unwrap();
        assert_eq!(stats.blocks_processed, 2);
        assert_eq!(stats.coins_added, 1);
        assert_eq!(stats.coins_removed, 1);
        assert_eq!(stats.end_muhash, MuHash::new().to_bytes());
        assert_eq!(stats.end_height, 1);
        assert_eq!(stats.tree_size, 2);
    }

    #[test]
    fn test_builder_rejects_height_gap() {
        let first = delta(0, 1, vec![[0x11; 32]]);
        let second = linked_delta(2, first.block_hash, 2, vec![[0x22; 32]]);

        let err = process_stripped_blocks(vec![first, second]).unwrap_err();
        assert!(err.to_string().contains("height mismatch"));
    }

    #[test]
    fn test_builder_rejects_prev_hash_mismatch() {
        let first = delta(0, 1, vec![[0x11; 32]]);
        let second = linked_delta(1, [0x99; 32], 2, vec![[0x22; 32]]);

        let err = process_stripped_blocks(vec![first, second]).unwrap_err();
        assert!(err.to_string().contains("prev_block_hash mismatch"));
    }

    #[test]
    fn test_jsonl_reader() {
        let mut block = delta(0, 1, vec![[0x11; 32]]);
        block.additions.push(coin(9));
        let line = format!(
            concat!(
                "{{",
                "\"height\":0,",
                "\"header\":\"{}\",",
                "\"block_hash\":\"{}\",",
                "\"txids\":[\"{}\"],",
                "\"additions\":[{{",
                "\"txid\":\"{}\",",
                "\"vout\":0,",
                "\"amount\":1000,",
                "\"script_pubkey\":\"51\",",
                "\"height\":9,",
                "\"is_coinbase\":false",
                "}}]",
                "}}\n"
            ),
            hex::encode(block.header),
            hex::encode(block.block_hash),
            hex::encode(block.txids[0]),
            hex::encode(block.additions[0].txid)
        );

        let stats = process_jsonl_reader(io::Cursor::new(line)).unwrap();
        assert_eq!(stats.blocks_processed, 1);
        assert_eq!(stats.txids_checked, 1);
        assert_eq!(stats.coins_added, 1);
    }

    #[test]
    fn test_jsonl_reader_derives_txids_from_stripped_txs() {
        let stripped_tx = hex::decode("01000000000000000000").unwrap();
        let txid = sha256d(&stripped_tx);
        let block = delta(0, 1, vec![txid]);
        let line = format!(
            concat!(
                "{{",
                "\"height\":0,",
                "\"header\":\"{}\",",
                "\"block_hash\":\"{}\",",
                "\"stripped_txs\":[\"{}\"],",
                "\"additions\":[],",
                "\"removals\":[]",
                "}}\n"
            ),
            hex::encode(block.header),
            hex::encode(block.block_hash),
            hex::encode(stripped_tx)
        );

        let stats = process_jsonl_reader(io::Cursor::new(line)).unwrap();
        assert_eq!(stats.blocks_processed, 1);
        assert_eq!(stats.txids_checked, 1);
    }

    #[test]
    fn test_jsonl_reader_rejects_malformed_header_length() {
        let line = format!(
            concat!(
                "{{",
                "\"height\":0,",
                "\"header\":\"00\",",
                "\"block_hash\":\"{}\",",
                "\"txids\":[\"{}\"]",
                "}}\n"
            ),
            hex::encode([0u8; 32]),
            hex::encode([0x11; 32])
        );

        let err = process_jsonl_reader(io::Cursor::new(line)).unwrap_err();
        assert!(err.to_string().contains("field header"));
        assert!(err.to_string().contains("expected 80"));
    }

    #[test]
    fn test_binary_reader_uses_stripped_txs_for_header_chain() {
        let stripped_tx1 = vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let stripped_tx2 = vec![0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let txid1 = sha256d(&stripped_tx1);
        let txid2 = sha256d(&stripped_tx2);
        let mut first = delta(0, 1, vec![txid1]);
        first.additions.push(coin(3));
        let second = linked_delta(1, first.block_hash, 2, vec![txid2]);

        let mut bytes = Vec::new();
        binary_header_before_genesis(&mut bytes);
        write_binary_block(&mut bytes, &first, &[stripped_tx1]);
        write_binary_block(&mut bytes, &second, &[stripped_tx2]);

        let stats = process_binary_reader(io::Cursor::new(bytes)).unwrap();
        assert_eq!(stats.blocks_processed, 2);
        assert_eq!(stats.txids_checked, 2);
        assert_eq!(stats.coins_added, 1);
        assert_eq!(stats.end_height, 1);
        assert_eq!(stats.end_block_hash, second.block_hash);
    }

    #[test]
    fn test_attestation_size() {
        let stats = process_stripped_blocks(vec![delta(0, 1, vec![[0x11; 32]])]).unwrap();
        let att = produce_attestation(&stats);
        assert_eq!(att.len(), 914);
    }

    #[test]
    fn test_job_attestation_size_and_binding_slot() {
        let stats = process_stripped_blocks(vec![delta(0, 1, vec![[0x11; 32]])]).unwrap();
        let job_sha256 = [0x7bu8; 32];
        let att = produce_job_attestation(
            &stats,
            JobAttestationContext {
                job_version: 1,
                job_sha256,
            },
        );
        assert_eq!(att.len(), 916);
        assert_eq!(u16::from_le_bytes(att[0..2].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(att[2..4].try_into().unwrap()), 1);
        assert_eq!(&att[76..108], &job_sha256);
    }
}
