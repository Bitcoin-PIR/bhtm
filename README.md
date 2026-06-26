# Block Hash -> MuHash Binding

This crate is the slim MuHash builder for BitcoinPIR.

It does not run a Bitcoin node, parse full blocks, maintain a UTXO set, or
validate consensus. It consumes an externally prepared stream of stripped block
deltas and attests the narrow binding that this component is responsible for:

```text
trusted block header hash
  -> 80-byte block header
  -> previous-header hash link to the trusted start anchor
  -> txid Merkle root over stripped transaction commitments
  -> supplied Bitcoin Core-compatible coin additions/removals
  -> full 384-byte MuHash value
  -> append-only MuHash Merkle tree leaf
```

The data source machine is responsible for deriving the coin deltas from Bitcoin
Core semantics, including undo data, duplicate txid/BIP30 handling, genesis
exclusion, unspendable outputs, height, and coinbase metadata. The MuHash builder
only applies the supplied deltas.

For non-genesis continuation, the builder trusts a start anchor:
`start_height`, `start_block_hash`, and `start_muhash`, where `start_muhash` is
the normalized 384-byte MuHash value. The 32-byte SHA256 commitment is still
available as a helper for Bitcoin Core comparisons, but the run endpoints keep
the full value.

## Quick Start

```bash
cd /Users/cusgadmin/bitcoin-pir/blockhash-to-muhash
cargo test
cargo build --release
```

Process a compact binary stripped-block stream:

```bash
target/release/blockhash-to-muhash bin stripped-deltas.bin
prepare-deltas ... | target/release/blockhash-to-muhash bin -
```

Write proof artifacts for a TEE wrapper or plain-host reproducibility check:

```bash
target/release/blockhash-to-muhash \
  bin stripped-deltas.bin \
  --expect-end-core-muhash <bitcoin-core-display-muhash> \
  --attestation-out proof/attestation.bin \
  --report-data-out proof/report-data.bin \
  --stats-json-out proof/stats.json
```

`attestation.bin` is the byte payload intended to be signed or bound into a
TEE quote. `report-data.bin` is 64 bytes:
`SHA512("BitcoinPIR/blockhash-to-muhash/report-data/v1" || attestation.bin)`.
`stats.json` records the same anchors, full 384-byte endpoint MuHash,
Core-display MuHash commitment, tree root, `attestation_sha256`, and
`tee_report_data`.

For the reusable generic SEV-SNP proof-runner UKI, put chunk-specific values in
runtime `job.json` instead of the UKI cmdline:

```bash
target/release/blockhash-to-muhash canonical-job job.json > proof/job.json
target/release/blockhash-to-muhash job-env proof/job.json
target/release/blockhash-to-muhash job-teed proof/job.json stripped-deltas.bin
```

`job-teed` emits attestation v2. It binds
`job_sha256 = SHA256(canonical_job_json)` into `attestation.bin`; its
`report-data.bin` uses
`SHA512("BitcoinPIR/blockhash-to-muhash/report-data/v2" || attestation.bin)`.
This is the path used by `scripts/build_bhtm_proof_uki.sh` to keep
one stable SEV-SNP `MEASUREMENT` across different chunk jobs.

JSONL input remains available as a debug/dev convenience:

```bash
target/release/blockhash-to-muhash jsonl stripped-deltas.jsonl
```

## Non-TEE VPSBG Smoke

The small real-data smoke used during bring-up processes blocks `0..9999` from
the compact binary stream generated from the local Bitcoin Core node. It checks
the end state against Core's historical `gettxoutsetinfo muhash 9999 true`
result.

```bash
scripts/vpsbg-nontee-smoke.sh
```

Defaults:

- host: `vpsbg-pir`
- remote workdir: `/root/blockhash-to-muhash-nontee`
- input stream: `/tmp/bhtm_first_10000.bin`
- expected Core MuHash:
  `fa8bc3bb3e3fb84e219da85b381cf140df1ee29d044272a6cac86e4a75c25cd4`

Measured VPSBG result from the first run:

```text
end_height: 9999
end_block_hash_display: 00000000fbc97cc6c599ce9c24dd4a2243e2bfd518eda56e1d5e47d29e29c3a7
end_muhash_core_display: fa8bc3bb3e3fb84e219da85b381cf140df1ee29d044272a6cac86e4a75c25cd4
tree_root: 1dec3c7c24630ba668e3b5718fd82b5dc83129092f3333e9ee9062e10059371f
attestation_size: 914
attestation_sha256: 2428bea5ee74e12c6eda0943cc8f6a85b0775ff5df6d59ffd899489ef6d548cb
tee_report_data: f7b8e78dba4774341a3fb1218af65eedee766f09204b5a0c2ebc1ae79103ee91dc056161aaae1da4769982081527edcb4b51ded4ed9ea590fab5c875fb59e499
wall_time: 2:08.06
max_rss: 2396 KiB
```

## First-100k Streaming Verification

The source-side Core RPC exporter can materialize or stream compact binary
deltas:

```bash
scripts/export-core-deltas.py \
  --end-height 99999 \
  --out /tmp/bhtm_first_100000.bin \
  --stats-out /tmp/bhtm_first_100000.export.stats.json
```

The first-100k VPSBG verification was run by piping the compact stream over SSH
into the remote verifier's stdin. The compact stream was not staged as a remote
input file; only proof artifacts were written under
`/root/blockhash-to-muhash-nontee/proof-runs/first-100000-stream/`.

Core reference:

```text
height: 99999
bestblock: 000000000002d01c1fccc21636b607dfd930d31d01c3a62104612a1719011250
txouts: 71885
muhash: f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4
```

Source/export measurements:

```text
raw block input bytes: 59754905
compact stream bytes: 107356923
compact stream sha256: 612c3d62a3097151d4b7fed89a588bf541a797cc20dd96d13337ffe67a209384
export wall time: 14.57s
transactions: 216573
coins_added: 264247
coins_removed: 192362
final_utxos: 71885
duplicate_overwrites: 2
duplicate_overwrite_sats: 10000000000
```

Remote verification result:

```text
stream wall time: 1368.72s
remote verifier wall time: 22:46.04
remote verifier max_rss: 2948 KiB
end_height: 99999
end_block_hash_display: 000000000002d01c1fccc21636b607dfd930d31d01c3a62104612a1719011250
end_muhash_core_display: f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4
tree_root: 4ae1b5fcc5eebedefbbd9b8a28282c524601ee48ac74814b1b48e3f3bedec0ce
attestation_sha256: 657b823f80dd031134534331197192bdd265900155b31c3262f62530f930285c
tee_report_data: 49b9f9756cd2ce58011c0387cb6a911ab924feddbc3c6635085632b59ce732e5ea6c0d98cbbe1eb6ea049b33e54cacb8f4696660dd274d1d91c3803e64d45ada
```

## Rolling 100k Stripped-Delta Archive

Long-term chunk storage is on Hetzner:
`pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/`.

The local staging directory is:
`/Volumes/Bitcoin/data/blockhash-to-muhash/stripped-deltas/`.
Use it only for the active chunk; after upload, remote hash verification, and
`zstd -t`, delete the local chunk files.

Uploaded chunks:

```text
mainnet-000000-099999/
  stream.bhtmbin.zst bytes: 45016264
  stream.bhtmbin.zst sha256: 4b63feb2d16664438a7b2d793e60de16a98922b08dc83e2dab491c60b7311b5a
  uncompressed bytes: 107356923
  uncompressed sha256: 612c3d62a3097151d4b7fed89a588bf541a797cc20dd96d13337ffe67a209384
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-100000-199999/
  stream.bhtmbin.zst bytes: 2508868002
  stream.bhtmbin.zst sha256: ad200d135aaff0eb06707c52d51a5d50b6024689746ca57a0bfc88f86f436f57
  uncompressed bytes: 5540234865
  uncompressed sha256: a9e944855f2286902fa45bfdd3c78a9b5aa9c5b5ed2a20a638f17be63e3403b0
  verified Core MuHash: e1e783e9ac237240789cb97cf37b8e108c038b44b07e9c58768233bfb1a80f06
  verifier elapsed: 334.64s
  verifier attestation sha256: 3e4516758043ec2605ff615896912ff4b94a8051675ccb464ee9235c6de56e88
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-200000-299999/
  stream.bhtmbin.zst bytes: 13244754182
  stream.bhtmbin.zst sha256: f10d30119ad5d3f00944271dd9e1ca64bd73a508128d873fa96e28c34586bbb5
  uncompressed bytes: 27856464980
  uncompressed sha256: 42f928f9eca9fc6f5eef8ff45a0a00624269c30fea7c37fa84c288467b57e5f5
  verified Core MuHash: c8df37777612a7d513ffce0d17921cd0bc43e375f40f5f374c829dfbbd6cd3a0
  verifier elapsed: 1703.63s
  verifier attestation sha256: 8fe357b5098531cd49ebfe1f026f92886a0f35bc7e1c2489a478c26718fc9ea4
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-300000-399999/
  stream.bhtmbin.zst bytes: 40195972526
  stream.bhtmbin.zst sha256: dee863e6f3f7274b3d46480d2da7f7161aa701efe24295a2f0fa62e581d4eed4
  uncompressed bytes: 76733738399
  uncompressed sha256: 1031a252864521407d9828533fa6bfeaa56f34b5fc12d3f0caa25f65da033360
  verified Core MuHash: 18cf1650a245e0979bfeb64495d04e39fa1dd1fbefca29aeb3bdecae7631ade1
  verifier elapsed: 1540.86s
  verifier tree root: 31516b07223100f319690f1b057227960549638cd879a311822f659c62fdeb2b
  verifier attestation sha256: db018be0cfe67f1ed935a7c7e1f3cb73153235c941e6d81df0fd03d7c4e61298
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-400000-499999/
  stream.bhtmbin.zst bytes: 93188176299
  stream.bhtmbin.zst sha256: 0904a153f65cfdb2204357af04ceca469147db8ed8e67f97868e06015ffc0178
  uncompressed bytes: 152393244287
  uncompressed sha256: c0d80facd804f5b43b439636bef38d984dbd940ee867d193c2da07d33245970b
  verified Core MuHash: 171e08e48bad8674528fb8d1a6fd9de42b4cfcdf78d8caf6558016fb9cb2a4ed
  verifier elapsed: 3027.89s
  verifier tree root: 38a0d50733e5209dcc9de2ab8e27551b3edf975e8dd96fdbc7eac52cf09a054c
  verifier attestation sha256: 355c7968f0cb8ecebc728f52ffcc5d9443ccdaef27387b0218563ee96fbfffe3
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-500000-599999/
  stream.bhtmbin.zst bytes: 83295545724
  stream.bhtmbin.zst sha256: 21bd7936a31c9c001156248927673a5c59a2a5fdb42d73528036065c9017c5fe
  uncompressed bytes: 145439276144
  uncompressed sha256: d00ce37fcc9e791a64afb3687690dddf2ab10df6569716cb51dc8bf56d7dd1fb
  verified Core MuHash: 53882696420706cc51cedb6082413569196f667d9d4b8d87d6dc2a9abd6ac4ed
  verifier elapsed: 3117.18s
  verifier tree root: f6abf2e16557a6ef9dd7373e955360ac1a1293f919cab65d2fee96224cabb08a
  verifier attestation sha256: 01d130858aa5f9edbc3683130e1204e2746eb27e1afed3a564755d94f84cd664
  manifest sha256: 58ddbd1e27650fc56b0096191f7a4454618cd283ce2ec0b85f05a7f03d15c7f3
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-600000-699999/
  stream.bhtmbin.zst bytes: 96329808333
  stream.bhtmbin.zst sha256: d4cdc4f189cd8f7ad66073d15a0d4900e4644b999b15f8946af698c9ddc07849
  uncompressed bytes: 166494341612
  uncompressed sha256: ba7282db8c2ecd40eea384d3b84c2256c10545285241fa64280168d464a11833
  verified Core MuHash: 5382e786ee8a1cc4e92d59030295e9cc0d3c0e3aea9c66e2fb9ee7cdae9d6f55
  verifier elapsed: 3832.89s
  verifier tree root: a94b44aa8abbd882637ddc8fc9b798d0ae098e1422c2a9d50c10ce11aaf23a12
  verifier attestation sha256: c76b04f4e0ba5b06f6d571dd1da71537d90f30019b9a3c18310bbb9655c8700d
  manifest sha256: 052492d0be6536bdf771fb83a1f0fa3dde751655e7a6e2b9ca3050009b9db443
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-700000-799999/
  stream.bhtmbin.zst bytes: 82688792803
  stream.bhtmbin.zst sha256: 706add584c24296b2548b63494a0858eb14c55c4b1a3e3e2718a7f5247152142
  uncompressed bytes: 159115007032
  uncompressed sha256: 6526a6bd489230e6424773ae81cddf02f57f5b45b9d67c01cb18198ea93dc0ad
  verified Core MuHash: 9226d85add58fe2635c468013944cdefb8fb8fae995888ea76621ff05ffc1c21
  verifier elapsed: 3961.08s
  verifier tree root: ac3762584df067f34e46fbe5ad8ddc6ec3ead3d7073bc2d5f73fc06fbdd3411b
  verifier attestation sha256: a931b22c893f7eec346aad8407c1e7cc12198bcadec935cb3ebee5c8b1107546
  manifest sha256: f2d76737534aa997a121127602806bde46f40e8b7a5901af9c4ed38edac8ccee
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-800000-899999/
  stream.bhtmbin.zst bytes: 81289881114
  stream.bhtmbin.zst sha256: efa4c3e14540e9777cc214159bceab74e82ba4ec13def3059e85bf3879d3a758
  uncompressed bytes: 198425503154
  uncompressed sha256: db16be1463954c2ba260c222006094640450682c1fc2fb835b0452fca1521e14
  verified Core MuHash: 1ec8dedf1473bd17f421c1dd7c78d6ba61747ac3161c14a27322b528bb032ca9
  verifier elapsed: 5097.41s
  verifier tree root: e3d61a7e18dc74ea798470917e40d8e578de946fe32e13022a8d986236dcc14b
  verifier attestation sha256: 4a95f3622ba857b7b5fea79fdd21e24740609d0ff40711d423a885d1e53d2b37
  manifest sha256: 410aad5211e7e0e04f633383c64f6aa2d5aa8e40186043befdb7dedf09d42297
  verifier end full MuHash is in manifest.json and is the next chunk anchor.

mainnet-900000-954920/
  current-head partial chunk, selected as a conservative tip-minus-6 anchor.
  This is not a canonical 100k boundary chunk.
  stream.bhtmbin.zst bytes: 40599117380
  stream.bhtmbin.zst sha256: 432510fbcf72d53cf9bd08b2161c75aadc4edecf46cdfccc557858a9ff73776a
  uncompressed bytes: 99504614791
  uncompressed sha256: 0dc2718e186803fc77b4b3536d79802b8ff3407d04342f98e8a4580da5bd3a03
  verified Core MuHash: c27e1fafa9dd154cfd16a78429477181febbecf49b3d49016c462b0764429470
  verifier elapsed: 2599.17s
  verifier tree root: babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6
  verifier attestation sha256: 667605a6fe956cc3379d08a31dfefb7b7bdd03846d4440fd0e69466389c75759
  manifest sha256: ab90de7b0744c86226fa14d3b3905b21e520933d9f5e0c45668e2e0243f5d45e
  verifier end full MuHash is in manifest.json and is the next chunk anchor.
```

For chunk 0, use `utxo-scan` from genesis so the historical BIP30 duplicate
txid overwrites are represented correctly. For later chunks, use Core's
verbosity-3 prevout data so no source-side UTXO map or snapshot is needed:

```bash
scripts/export-core-deltas.py \
  --start-height 100000 \
  --end-height 199999 \
  --source-mode prevout-rpc \
  --anchor-height 99999 \
  --anchor-block-hash 000000000002d01c1fccc21636b607dfd930d31d01c3a62104612a1719011250 \
  --anchor-muhash "$FULL_384_BYTE_MUHASH_FROM_PREVIOUS_VERIFIER" \
  --out /Volumes/Bitcoin/data/blockhash-to-muhash/stripped-deltas/mainnet-100000-199999/stream.bhtmbin \
  --stats-out /Volumes/Bitcoin/data/blockhash-to-muhash/stripped-deltas/mainnet-100000-199999/export.stats.json
```

The next standalone chunk needs the previous chunk's full 384-byte verifier
`end_muhash`, not only Bitcoin Core's 32-byte displayed MuHash digest.
`mainnet-900000-954920` is a verified current-head partial archive; use its
manifest `end_muhash` for continuation from height `954921`. A later canonical
`mainnet-900000-999999` archive should be regenerated or treated as a distinct
artifact.

## Input Contract

The production input is compact binary. It intentionally carries raw stripped
transaction bytes, not just txids, so the builder can recompute txids and verify
the block Merkle root and header chain itself.

```text
file:
  magic: "BHTMBIN1"
  anchor_kind: u8                 // 0 = before genesis, 1 = trusted block
  start_height: u32le
  start_block_hash: [u8; 32]
  start_muhash: [u8; 384]         // normalized MuHash value
  blocks...

block:
  height: u32le
  header: [u8; 80]
  block_hash: [u8; 32]
  tx_count: u32le
  repeat tx_count:
    stripped_tx_len: u32le
    stripped_tx: [u8; stripped_tx_len]
  addition_count: u32le
  coins...
  removal_count: u32le
  coins...

coin:
  txid: [u8; 32]
  vout: u32le
  amount: u64le
  height: u32le
  is_coinbase: u8                 // 0 or 1
  script_pubkey_len: u32le
  script_pubkey: [u8; script_pubkey_len]
```

The builder derives each txid as `SHA256d(stripped_tx)`. This is important for
security: `header.prev_block_hash`, `SHA256d(header)`, and the txid Merkle root
are all checked inside the builder.

### JSONL Debug Format

Each JSONL line is one stripped block delta:

```json
{
  "height": 948454,
  "header": "<80-byte header hex>",
  "block_hash": "<32-byte internal-order block hash hex>",
  "txids": ["<32-byte internal-order txid hex>"],
  "stripped_txs": ["<optional non-witness transaction bytes hex>"],
  "additions": [
    {
      "txid": "<32-byte internal-order txid hex>",
      "vout": 0,
      "amount": 5000000000,
      "script_pubkey": "51",
      "height": 948454,
      "is_coinbase": true
    }
  ],
  "removals": []
}
```

`adds`/`added` and `removes`/`removed` are accepted as aliases for
`additions` and `removals`.

`txids` are the minimal input. If `stripped_txs` is supplied instead, the builder
computes `SHA256d(stripped_tx)` for each entry and uses those txids for the
Merkle-root check. If both are supplied, they must match.

## What The Builder Verifies

For each block delta:

1. `SHA256d(header) == block_hash`
2. `height == previous_height + 1`
3. `header.prev_block_hash == previous_block_hash`
4. `Merkle(txids) == header.merkle_root`, deriving txids from `stripped_txs`
   when raw stripped transaction bytes are supplied
5. Apply every `addition` to MuHash.
6. Apply every `removal` to MuHash.
7. Append `(height, block_hash, muhash_value)` to the MuHash Merkle tree.

This is intentionally the full guarantee. The builder does not check:

- proof of work or difficulty retargeting
- transaction scripts, signatures, or witness data
- coinbase maturity or subsidy
- input membership in a UTXO set
- duplicate spends
- BIP30 or duplicate txid rules
- OP_RETURN/unspendable filtering

Those rules belong in the data source / delta extractor. This keeps the attested
MuHash component small and makes the trust boundary explicit.

## MuHash Details

MuHash follows Bitcoin Core's coinstats serialization:

```text
COutPoint || uint32(height << 1 | coinbase) || CTxOut
```

The hash-to-element path is also Core-compatible:

```text
SHA256(serialized_coin) -> ChaCha20(key, zero nonce) -> 384-byte LE Num3072
```

The exposed `start_muhash` and `end_muhash` values are the full normalized
384-byte accumulator values. `MuHash::commitment()` returns Bitcoin Core's
32-byte SHA256 commitment when a Core-style comparison is needed.

Because duplicate txids exist in Bitcoin history, the delta extractor must emit
exactly the coins Bitcoin Core would add/remove. The builder does not attempt to
recover those semantics locally.

Removals stay in the MuHash denominator, matching Bitcoin Core's `MuHash3072`
shape. The streaming builder snapshots each block's un-inverted
`(numerator, denominator)` accumulator and batch-inverts pending denominators
before appending the per-block full MuHash leaves to the tree. This preserves
the exact per-block 384-byte MuHash values while avoiding one modular inversion
per block.

## Architecture

```text
src/lib.rs         public modules
src/main.rs        CLI: bin <path|->, jsonl <path|-> for debug/dev
src/stream.rs      stripped block delta verification + slim builder loop
src/muhash.rs      Bitcoin Core-compatible MuHash
src/num3072.rs     3072-bit modular arithmetic
src/merkle_tree.rs append-only MuHash binding tree
```

The compiled crate no longer depends on `bitcoin`, raw `blk*.dat` readers, RPC
fetching, chain validation, or an in-memory UTXO `HashMap`.

## Storage Model

The builder does not need local block storage and does not store the UTXO set.
It keeps only:

- the MuHash accumulator
- the MuHash Merkle tree frontier
- counters and last-block metadata

Large storage belongs on the data source machine, which can colocate with
Bitcoin Core and stream prepared stripped block deltas to this builder.

## Tests

```bash
cargo test
```

Current coverage: 41 tests.
