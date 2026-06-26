# Stripped Input Size Notes

The old compression question was: how much raw block data can be removed before
the TEE can no longer verify `block_hash -> txid Merkle root`?

For the selected slim builder, the input is smaller and more explicit:

```text
80-byte header
32-byte trusted block_hash
raw non-witness transaction bytes, so the builder derives txids itself
Core-compatible coin additions
Core-compatible coin removals
```

The builder receives raw stripped transaction bytes and hashes them as opaque
bytes. This lets it verify the header's txid Merkle commitment itself while
still avoiding transaction parsing and script validation:

```text
Merkle(txids) == header.merkle_root
SHA256d(header) == block_hash
```

## What Moved To The Data Source

The data source still needs enough Bitcoin Core data to derive the deltas:

- full block data or decoded transactions for new outputs
- undo data / chainstate for spent coin metadata
- Bitcoin Core's coinstats exclusions and historical duplicate txid behavior

That source-side work may use stripped non-witness blocks as an optimization,
but that is no longer part of the attested builder's storage or memory budget.

## Builder Bandwidth

The per-block builder stream is dominated by:

- `32 * tx_count` bytes for txids
- serialized added coins
- serialized removed coins

For removals, this is necessarily larger than transaction inputs alone because
MuHash removal requires the full spent coin:

```text
txid, vout, amount, script_pubkey, height, is_coinbase
```

This is the price of keeping the builder stateless with respect to the UTXO set.
