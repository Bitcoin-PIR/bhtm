# Separate Proof Shape

The block-hash-to-MuHash proof is separate from the PIR database build proof.
Its job is to produce an append-only commitment over `(height, block_hash,
muhash)` leaves.

## Builder Input

The builder consumes prepared stripped block deltas, not raw Bitcoin blocks:

```text
header + trusted block_hash + stripped_txs + additions + removals
```

The compact binary format carries raw stripped transaction bytes. The builder
derives txids by `SHA256d(stripped_tx)` and does not parse them further. It then
checks the txid Merkle root, height continuity, and `header.prev_block_hash` from
the trusted start anchor. The additions/removals are enough to update MuHash
without maintaining a UTXO set.

## Builder Output

For a run, the builder outputs:

```text
version
start_height
end_height
start_block_hash
end_block_hash
start_muhash [384B]
prev_muhash_tree_root
new_muhash_tree_root
tree_size
end_muhash [384B]
```

The SEV report data can be:

```text
SHA256("BitcoinPIR/blockhash-to-muhash/v1\0" || attestation_bytes)
```

## Composition

Adjacent runs compose by checking:

```text
run_a.end_height + 1 == run_b.start_height
run_a.new_muhash_tree_root == run_b.prev_muhash_tree_root
```

This proves append-only continuity of the MuHash tree roots. It does not prove
proof of work, difficulty, or Bitcoin transaction validity; those remain outside
this builder.

## Data Source Responsibility

The data source should be close to Bitcoin Core and should emit the exact coin
deltas that Bitcoin Core coinstats would use:

```text
COutPoint || uint32(height << 1 | coinbase) || CTxOut
```

For removals, the data source must use undo/chainstate information to provide
the full spent coin. Transaction inputs alone only contain `(prev_txid, vout)`,
which is insufficient for MuHash removal.

## Storage

The builder has no block storage and no UTXO storage requirement. It can stream
JSONL from stdin and keep only the MuHash accumulator plus the Merkle frontier.
