# Slim Block Hash -> MuHash Binding

This document records the selected design.

Earlier sketches had the TEE parse stripped Bitcoin blocks, validate header
chains, and maintain an in-memory UTXO set. That design is superseded. The
MuHash builder is now deliberately narrower: it receives stripped block deltas
that are already prepared by a data source colocated with Bitcoin Core.

## Trust Boundary

The MuHash builder verifies only:

```text
SHA256d(80-byte header) == trusted block_hash
header.prev_block_hash   == previous trusted/processed block_hash
height                   == previous height + 1
Merkle(txids)           == header.merkle_root
```

It then applies the supplied coin additions/removals to the Bitcoin
Core-compatible MuHash accumulator and appends:

```text
H("muhash-leaf" || height || block_hash || muhash_value)
```

to the MuHash Merkle tree.

The builder does not validate:

- proof of work or difficulty adjustment
- scripts, signatures, or witness
- coinbase maturity or subsidy
- UTXO membership
- double spends
- BIP30 / duplicate txid behavior
- genesis or unspendable-output exclusions

Those are data-source responsibilities.

The builder can start either before genesis or from a trusted anchor. A trusted
anchor consists of `start_height`, `start_block_hash`, and the normalized
384-byte `start_muhash` accumulator value. The 32-byte SHA256 commitment remains
available for Core-style comparisons, but the builder's run endpoints use full
MuHash values.

## Input Record

The production stream is compact binary. Each stripped block delta contains:

```text
height:      u32
header:      [u8; 80]
block_hash:  [u8; 32]
stripped_txs: Vec<Vec<u8>>
additions:   Vec<Coin>
removals:    Vec<Coin>
```

`Coin` is the exact object used by Bitcoin Core coinstats MuHash:

```text
txid:          [u8; 32]
vout:          u32
amount:        u64
script_pubkey: Vec<u8>
height:        u32
is_coinbase:   bool
```

The builder computes `SHA256d(stripped_tx)` and uses those txids for the
Merkle-root check. The bytes are otherwise opaque; the builder does not parse
transaction structure. Supplying raw stripped transaction bytes, rather than
only txids, is part of the security boundary because the builder verifies the
block relationship against the header itself.

The source must use Core-compatible undo/coinstats semantics when producing
`additions` and `removals`. This is especially important for historical
duplicate txids/BIP30: the builder does not keep enough state to rediscover
which overwritten or skipped coins Core would include.

## Why This Is Low Storage

The builder does not store raw blocks and does not store the UTXO set. Runtime
state is only:

- MuHash numerator/denominator
- MuHash Merkle tree frontier
- counters and last processed metadata

Large disk use remains on the data source machine, where Bitcoin Core already
has block files, chainstate, and undo data.

## Security Statement

The builder attests:

> Given these header-bound stripped transaction commitments and these supplied
> Core-format coin deltas, this 384-byte MuHash value and MuHash tree root were
> computed.

It does not attest:

> These coin deltas are the unique consensus-correct Bitcoin UTXO transition for
> the block.

That stronger statement belongs either in the data source, an extractor proof,
or a separate verification component.
