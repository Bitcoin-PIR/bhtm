//! Integration tests for the slim stripped-block MuHash builder.

use blockhash_to_muhash::muhash::{MuHash, UtxoEntry};
use blockhash_to_muhash::stream::{
    block_hash_from_header, merkle_root_from_txids, process_stripped_blocks,
    process_stripped_blocks_from_anchor, SlimMuHashBuilder, StrippedBlockDelta, TrustedAnchor,
};
use std::time::Instant;

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

fn coin(tag: u8) -> UtxoEntry {
    UtxoEntry {
        txid: [tag; 32],
        vout: 0,
        amount: 1000 * tag as u64,
        script_pubkey: vec![0x51],
        height: tag as u32,
        is_coinbase: false,
    }
}

#[test]
fn test_processes_header_bound_stripped_deltas() {
    let mut first = delta(0, 1, vec![[0x11; 32]]);
    first.additions.push(coin(1));
    let mut second = linked_delta(1, first.block_hash, 2, vec![[0x22; 32], [0x33; 32]]);
    second.additions.push(coin(2));

    let stats = process_stripped_blocks(vec![first, second]).unwrap();
    assert_eq!(stats.blocks_processed, 2);
    assert_eq!(stats.txids_checked, 3);
    assert_eq!(stats.coins_added, 2);
    assert_eq!(stats.coins_removed, 0);
    assert_eq!(stats.end_height, 1);
    assert_eq!(stats.tree_size, 2);
    assert_ne!(stats.end_muhash, MuHash::new().to_bytes());
}

#[test]
fn test_remove_uses_supplied_coin_without_utxo_state() {
    let spent_coin = coin(9);
    let mut first = delta(0, 1, vec![[0x11; 32]]);
    first.removals.push(spent_coin);

    let stats = process_stripped_blocks(vec![first]).unwrap();
    assert_eq!(stats.blocks_processed, 1);
    assert_eq!(stats.coins_added, 0);
    assert_eq!(stats.coins_removed, 1);
}

#[test]
fn test_rejects_stripped_block_not_bound_to_header_hash() {
    let mut block = delta(0, 1, vec![[0x11; 32]]);
    block.block_hash[31] ^= 0x01;

    let err = process_stripped_blocks(vec![block]).unwrap_err();
    assert!(err.to_string().contains("header hash mismatch"));
}

#[test]
fn test_rejects_stripped_block_not_bound_to_header_merkle_root() {
    let mut block = delta(0, 1, vec![[0x11; 32]]);
    block.txids.push([0x22; 32]);

    let err = process_stripped_blocks(vec![block]).unwrap_err();
    assert!(err.to_string().contains("txid merkle root mismatch"));
}

#[test]
fn test_trusted_anchor_updates_muhash_over_linked_stripped_blocks() {
    let start_height = 100;
    let start_block_hash = [0x42; 32];
    let start_muhash = MuHash::new().to_bytes();
    let anchor = TrustedAnchor::from_block(start_height, start_block_hash, start_muhash);

    let coin_a = coin(7);
    let coin_b = coin(8);
    let mut block_101 = linked_delta(101, start_block_hash, 1, vec![[0x11; 32]]);
    block_101.additions.push(coin_a.clone());
    let mut block_102 = linked_delta(102, block_101.block_hash, 2, vec![[0x22; 32], [0x33; 32]]);
    block_102.removals.push(coin_a);
    block_102.additions.push(coin_b.clone());

    let end_height = block_102.height;
    let end_block_hash = block_102.block_hash;
    let stats = process_stripped_blocks_from_anchor(anchor, vec![block_101, block_102]).unwrap();

    let mut expected_muhash = MuHash::new();
    expected_muhash.add(&coin_b);

    assert_eq!(stats.start_height, start_height);
    assert_eq!(stats.start_block_hash, start_block_hash);
    assert_eq!(stats.start_muhash, start_muhash);
    assert_eq!(stats.end_height, end_height);
    assert_eq!(stats.end_block_hash, end_block_hash);
    assert_eq!(stats.end_muhash, expected_muhash.to_bytes());
    assert_eq!(stats.blocks_processed, 2);
    assert_eq!(stats.coins_added, 2);
    assert_eq!(stats.coins_removed, 1);
}

#[test]
fn test_muhash_accumulator_determinism() {
    let entries: Vec<UtxoEntry> = (0..10).map(coin).collect();

    let mut mh1 = MuHash::new();
    let mut mh2 = MuHash::new();

    for entry in &entries {
        mh1.add(entry);
    }
    for entry in &entries {
        mh2.add(entry);
    }
    assert_eq!(mh1.to_bytes(), mh2.to_bytes());

    for entry in entries.iter().rev() {
        mh1.remove(entry);
    }
    assert_eq!(mh1.to_bytes(), MuHash::new().to_bytes());
}

#[test]
fn test_batched_streaming_tree_matches_direct_per_block_processing() {
    let coin_a = coin(1);
    let coin_b = coin(2);
    let coin_c = coin(3);

    let mut first = delta(0, 1, vec![[0x11; 32]]);
    first.additions.push(coin_a.clone());
    first.additions.push(coin_b.clone());

    let mut second = linked_delta(1, first.block_hash, 2, vec![[0x22; 32], [0x33; 32]]);
    second.removals.push(coin_a);
    second.additions.push(coin_c.clone());

    let mut third = linked_delta(2, second.block_hash, 3, vec![[0x44; 32]]);
    third.removals.push(coin_b);
    third.removals.push(coin_c);

    let blocks = vec![first, second, third];
    let streaming_stats = process_stripped_blocks(blocks.clone()).unwrap();

    let mut direct_builder = SlimMuHashBuilder::new();
    for block in &blocks {
        direct_builder.process_block(block).unwrap();
    }
    let direct_stats = direct_builder.stats(Instant::now());

    assert_eq!(streaming_stats.end_muhash, direct_stats.end_muhash);
    assert_eq!(streaming_stats.tree_root, direct_stats.tree_root);
    assert_eq!(streaming_stats.tree_size, direct_stats.tree_size);
}
