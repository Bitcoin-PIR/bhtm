# Trust Chain Check: Block Hash to MuHash to DB Roots

Date: 2026-06-26

This file records a concrete end-to-end check for the existing database build
anchor at Bitcoin mainnet height `948454`.

## Result

The chain verifies:

```text
Bitcoin block hash at height 948454
  -> full 384-byte MuHash leaf in the BHTM chunk tree
  -> BHTM SEV-SNP proof tree_root
  -> Core-display MuHash used by the DB proof
  -> attested-builder database proof
  -> bucket/onion database Merkle super-roots
```

## BHTM Leaf Proof

Generated from the archived stripped stream:

```text
chunk:      mainnet-900000-954920
height:     948454
leaf_index: 48454
tree_size:  54921
```

Artifacts:

```text
remote leaf proof:
  pir-hetzner:/root/blockhash-to-muhash/leaf-proofs/mainnet-900000-954920/height-948454.leaf-proof.json
remote verify output:
  pir-hetzner:/root/blockhash-to-muhash/leaf-proofs/mainnet-900000-954920/height-948454.verify.txt
local copy:
  /tmp/bhtm-leaf-mainnet-900000-954920-h948454.json
```

Hashes:

```text
leaf proof sha256:
  0a6d6631984a00635528ac9f1df3530469d238b5aaf8b4a3fdb4a4a0fa46a015
run log sha256:
  7c03a7d413aa71b6a48e9e06eb0214809559687c288cd38d7cdbe4e43e0147e4
verify output sha256:
  71e3d7abff72a88ad1d99332f49d326a4b2f8199eb63be6b4f2ac0fdd85d0142
temporary Linux proof-generator sha256:
  bd9396cd9e79cd6574054f3ae372b8953c35460d3395ad0cb940836a35a98910
```

Verified leaf contents:

```text
block_hash:
  00000000000000000001ef683c02c383315db7e917c69d20f79e05985560a4e4
core_muhash:
  cf4fc1f1dd400622a5b6f39eca7f764a30570c30cc668e04f00e8a3356c2a2ee
leaf_hash:
  0171a475dc43b97db594c7512039d12f5213d270b42e56babfa44d65359eeaaf
tree_root:
  babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6
proof_steps:
  16
```

The leaf proof verifies against the same `tree_root` committed by the
`mainnet-900000-954920` BHTM proof:

```text
proof remote:
  pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-900000-954920/proof-generic/
proof status:
  verified
verify-bhtm-quote:
  ok
AMD VCEK/report signature:
  ok
UKI measurement:
  652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
```

## DB Proof Link

The existing DB proof directory:

```text
/Users/cusgadmin/BitcoinPIR/deploy/attested-builder-runs/delta_940611_948454_sev_snp
```

was verified using the BHTM leaf proof's own `block_hash_display` and
`core_muhash_display` as expectations.

The DB proof accepted:

```text
height:
  948454
block_hash:
  00000000000000000001ef683c02c383315db7e917c69d20f79e05985560a4e4
muhash:
  cf4fc1f1dd400622a5b6f39eca7f764a30570c30cc668e04f00e8a3356c2a2ee
bucket_super_root:
  e2ba2eee6788424309a95f771893d5401cc8e3ceec6188dc2708900e211a910a
onion_super_root:
  f86baa3966a61cdcd70d8c0ad9bed233f591806eb351db2ae35ac0192a3fe997
build_evidence_sha256:
  977abca3ca8dc5dfce06d69006feeb6c0df5e7df3d7c1d758fc717254ff10697
sev_snp_report_data:
  a8e04cc4785c9bd7046463cbdfdd7ebf7ab588369130ff7ff8c1fc576d7c0a79e02fc12a2e1e7df038c4a58dc48b6f7d4c0a5676634826030dfed341143dfbda
```

Command output:

```text
/tmp/bhtm-db-chain-948454.verify.txt
```

## Verification Commands

Leaf proof verification:

```bash
target/release/blockhash-to-muhash verify-leaf-proof \
  /tmp/bhtm-leaf-mainnet-900000-954920-h948454.json \
  --expect-height 948454 \
  --expect-tree-root babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6
```

DB proof verification used the BHTM leaf proof's `block_hash_display` and
`core_muhash_display` as the expected DB anchor and MuHash.

Note: the BHTM proof path includes AMD VCEK/report-signature verification. The
existing DB proof verifier checks the build evidence, root-bundle payload,
manifest hashes, and SNP `REPORT_DATA` binding; it does not print an AMD
VCEK/signature verdict in this local smoke command.
