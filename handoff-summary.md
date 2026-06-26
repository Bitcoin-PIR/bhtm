# Blockhash-to-Muhash Handoff Summary

Date: 2026-06-23
Workspace: `/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash`
Data source used: `/Volumes/Bitcoin/bitcoin`
Archive destination: `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/`

## Current endpoint reached

Mainline archive work is complete through:
- **Height:** `954920` (partial head chunk, conservative tip-minus-6)
- **Chunk:** `mainnet-900000-954920`
- **Status:** verified end MuHash against local Core-derived expected value and archived to Hetzner
- **Next start height (if continuing):** `954921`

This is a **current-head partial**, not a canonical 100k boundary artifact.

## Architecture / behavior implemented and enforced

- Builder/verifier now consumes compact binary stripped deltas only.
- It validates chain linkage:
  - `SHA256d(header) == block_hash`
  - `height == previous_height + 1`
  - `header.prev_block_hash == previous_block_hash`
  - `Merkle(txids) == header.merkle_root`
- It computes txid by hashing raw stripped tx bytes directly (`SHA256d(stripped_tx)`), no transaction parsing.
- MuHash applies only Core-style additions/removals and appends `(height, block_hash, full_384_byte_muhash)` leaves.
- Streaming path uses un-inverted `(numerator, denominator)` snapshots per block and batch-inverts pending denominators before appending leaves (no per-block inversion).
- No UTXO set in builder; no raw `blk*.dat`; no RPC in builder.
- Duplicate-txid semantics are handled by the data source extractor (per BTC Core coin/coinstats rules), not by builder.

## Current rolling archive inventory

All listed entries were verified and (where applicable) manifested with **full 384-byte** verifier `end_muhash`.

| Chunk | Compressed bytes | Comp SHA256 | Uncompressed bytes | Uncompressed SHA256 | End Core MuHash | Verifier elapsed | Attestation SHA256 |
|---|---:|---|---:|---|---|---:|---|
| mainnet-000000-099999 | 45,016,264 | 4b63feb2d16664438a7b2d793e60de16a98922b08dc83e2dab491c60b7311b5a | 107,356,923 | 612c3d62a3097151d4b7fed89a588bf541a797cc20dd96d13337ffe67a209384 | in `mainnet-000000-099999/manifest.json` | n/a | manifest / stats only |
| mainnet-100000-199999 | 2,508,868,002 | ad200d135aaff0eb06707c52d51a5d50b6024689746ca57a0bfc88f86f436f57 | 5,540,234,865 | a9e944855f2286902fa45bfdd3c78a9b5aa9c5b5ed2a20a638f17be63e3403b0 | e1e783e9ac237240789cb97cf37b8e108c038b44b07e9c58768233bfb1a80f06 | 334.64 s | 3e4516758043ec2605ff615896912ff4b94a8051675ccb464ee9235c6de56e88 |
| mainnet-200000-299999 | 13,244,754,182 | f10d30119ad5d3f00944271dd9e1ca64bd73a508128d873fa96e28c34586bbb5 | 27,856,464,980 | 42f928f9eca9fc6f5eef8ff45a0a00624269c30fea7c37fa84c288467b57e5f5 | c8df37777612a7d513ffce0d17921cd0bc43e375f40f5f374c829dfbbd6cd3a0 | 1703.63 s | 8fe357b5098531cd49ebfe1f026f92886a0f35bc7e1c2489a478c26718fc9ea4 |
| mainnet-300000-399999 | 40,195,972,526 | dee863e6f3f7274b3d46480d2da7f7161aa701efe24295a2f0fa62e581d4eed4 | 76,733,738,399 | 1031a252864521407d9828533fa6bfeaa56f34b5fc12d3f0caa25f65da033360 | 18cf1650a245e0979bfeb64495d04e39fa1dd1fbefca29aeb3bdecae7631ade1 | 1540.86 s | db018be0cfe67f1ed935a7c7e1f3cb73153235c941e6d81df0fd03d7c4e61298 |
| mainnet-400000-499999 | 93,188,176,299 | 0904a153f65cfdb2204357af04ceca469147db8ed8e67f97868e06015ffc0178 | 152,393,244,287 | c0d80facd804f5b43b439636bef38d984dbd940ee867d193c2da07d33245970b | 171e08e48bad8674528fb8d1a6fd9de42b4cfcdf78d8caf6558016fb9cb2a4ed | 3027.89 s | 355c7968f0cb8ecebc728f52ffcc5d9443ccdaef27387b0218563ee96fbfffe3 |
| mainnet-500000-599999 | 83,295,545,724 | 21bd7936a31c9c001156248927673a5c59a2a5fdb42d73528036065c9017c5fe | 145,439,276,144 | d00ce37fcc9e791a64afb3687690dddf2ab10df6569716cb51dc8bf56d7dd1fb | 53882696420706cc51cedb6082413569196f667d9d4b8d87d6dc2a9abd6ac4ed | 3117.18 s | 01d130858aa5f9edbc3683130e1204e2746eb27e1afed3a564755d94f84cd664 |
| mainnet-600000-699999 | 96,329,808,333 | d4cdc4f189cd8f7ad66073d15a0d4900e4644b999b15f8946af698c9ddc07849 | 166,494,341,612 | ba7282db8c2ecd40eea384d3b84c2256c10545285241fa64280168d464a11833 | 5382e786ee8a1cc4e92d59030295e9cc0d3c0e3aea9c66e2fb9ee7cdae9d6f55 | 3832.89 s | c76b04f4e0ba5b06f6d571dd1da71537d90f30019b9a3c18310bbb9655c8700d |
| mainnet-700000-799999 | 82,688,792,803 | 706add584c24296b2548b63494a0858eb14c55c4b1a3e3e2718a7f5247152142 | 159,115,007,032 | 6526a6bd489230e6424773ae81cddf02f57f5b45b9d67c01cb18198ea93dc0ad | 9226d85add58fe2635c468013944cdefb8fb8fae995888ea76621ff05ffc1c21 | 3961.08 s | a931b22c893f7eec346aad8407c1e7cc12198bcadec935cb3ebee5c8b1107546 |
| mainnet-800000-899999 | 81,289,881,114 | efa4c3e14540e9777cc214159bceab74e82ba4ec13def3059e85bf3879d3a758 | 198,425,503,154 | db16be1463954c2ba260c222006094640450682c1fc2fb835b0452fca1521e14 | 1ec8dedf1473bd17f421c1dd7c78d6ba61747ac3161c14a27322b528bb032ca9 | 5097.41 s | 4a95f3622ba857b7b5fea79fdd21e24740609d0ff40711d423a885d1e53d2b37 |
| mainnet-900000-954920 | 40,599,117,380 | 432510fbcf72d53cf9bd08b2161c75aadc4edecf46cdfccc557858a9ff73776a | 99,504,614,791 | 0dc2718e186803fc77b4b3536d79802b8ff3407d04342f98e8a4580da5bd3a03 | c27e1fafa9dd154cfd16a78429477181febbecf49b3d49016c462b0764429470 | 2599.17 s | 667605a6fe956cc3379d08a31dfefb7b7bdd03846d4440fd0e69466389c75759 |

> Note: the table intentionally keeps `mainnet-000000-099999` first-end hash summarized in AGENTS/README; consult those files for any missing short fields.

For all chunks there are matching:
- `stream.bhtmbin.zst`
- `export.stats.json`
- `verify.stats.json`
- `attestation.bin`
- `report-data.bin`
- `manifest.json`

## 900000-954920 chunk full verifier end state

- End height: `954920`
- End block hash: `00000000000000000001b0c7d606f5da5d39262dbfceb848fe0d19337fb9bf17`
- End MuHash (full 384-byte):
`86068d99041876906125c9427ec07d1434190c8bf9399457c395a66e6d778790d1d8eceac9ca0fbeec9db16f4782cb19ac56b54918ce17ea790871da9f36fa6fa6c2185a7aa3e8b6743e2b71169c0cb88de13fa15b76bed39bb6e1ae1f7f225f0cff1a4ac6f7f91b3a59dda8c38b54036da760bc6431a4b6b84eb0702283db3b4b3043859da500147ee357d1c0991d35eb086169575d91a681ff6286344881f8fff7b421c92225526051f6236b75662be85f518515467ab742a8191cb7b8e79ecce28f731a0cc702d2cf37b7e74b96ed7d72aecc32bb3ae0aa55d6101a2fb7b2d621120ee23a1c6c329267902dbedbd677382258f93cdb551adbcb7073bc86ab30e54da465438af0da988e945e353871c90d97311f2c8a9ff8015faf305410e9e5adf2cedb1a4f974df1f82a9d16c476deff002ea192540fdc21752f9a1eac76e835d7644499f746abb74b1150074ff42f51da66357364bfd91dc306afb8dac1ea40c895fbe3172ddd551c7fb7b60574863f68915e361398b95dee87bede90d8`
- Core-comparison MuHash: `c27e1fafa9dd154cfd16a78429477181febbecf49b3d49016c462b0764429470`
- Tree root: `babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6`

## Continuation anchor for next chunk

Use this as trusted start for `954921`:
- `start_height`: `954920`
- `start_block_hash`: `00000000000000000001b0c7d606f5da5d39262dbfceb848fe0d19337fb9bf17`
- `start_muhash`: full 384-byte value from:
  - `mainnet-900000-954920/manifest.json`
  - (`start_muhash` field for next run)

Remote manifest path:
`/root/blockhash-to-muhash/stripped-deltas/mainnet-900000-954920/manifest.json`

Manifest sha256:
`ab90de7b0744c86226fa14d3b3905b21e520933d9f5e0c45668e2e0243f5d45e`

## Storage / workflow notes

- Data is kept only transiently in `/Volumes/Bitcoin/data/blockhash-to-muhash/stripped-deltas/` and removed after successful upload/verify.
- Remote file uploads were sent in 4-way parallel rsync of 2GiB parts (`split`), then reassembled and validated with:
  - `sha256sum stream.bhtmbin.zst`
  - `zstd -t stream.bhtmbin.zst`
- Builder/verifier binaries do **not** require large memory:
  - only MuHash state + Merkle frontier + metadata

## Verification status

- VPSBG smoke and early streaming runs were successful and match documented hashes.
- End-to-end verifier comparisons against Core MuHash were performed for all archived chunks listed above.
- Current tests run earlier:
  - `cargo test` (41 tests passed at that point)
  - `cargo test --test integration` (7 integration tests)

## Open follow-up for new session

- Continue from `mainnet-954921-...` if extending archive.
- Optionally regenerate canonical `mainnet-900000-999999` chunk if needed (current partial is non-canonical by design).
- Keep `manifest` as source-of-truth for next-anchor `start_muhash`.

## SEV-SNP attestation tooling (updated 2026-06-25)

The old per-chunk UKI path worked for chunk 0, but the forward design is now a
reusable generic UKI plus runtime `job.json`.

Current subcommands:

- `bin-teed <path|->` — legacy/direct mode. Emits attestation v1 unless a job
  context is present.
- `canonical-job <job.json>` — validates and writes canonical JSON.
- `job-env <job.json>` — validates and writes POSIX shell variables for the
  initramfs runner.
- `job <job.json> <path|->` — processes compact binary deltas using start/end
  expectations from the job.
- `job-teed <job.json> <path|->` — generic proof mode. Emits
  `proof/{job.json,attestation.bin,report-data.bin,sev-snp-report.bin,stats.json}`.
- `verify-bhtm-quote <att.bin> <rd.bin> <snp-report.bin>` — checks
  `report-data.bin == SHA512(domain || attestation.bin)` and the raw SEV-SNP
  REPORT_DATA field at offset `0x50`. It still does not verify the AMD ECDSA
  signature; pair with `BitcoinPIR/pir-attest-verify`.

Attestation versions:

- v1: 914 bytes, old per-chunk format, report-data domain
  `BitcoinPIR/blockhash-to-muhash/report-data/v1`.
- v2: 916 bytes, generic-UKI format. Adds `job_version` and replaces the old
  reserved 32-byte slot with `job_sha256`. Report-data domain
  `BitcoinPIR/blockhash-to-muhash/report-data/v2`.

Generic UKI path:

- Build script: `scripts/build_bhtm_proof_uki.sh`.
- Init script: `scripts/dracut/98bhtm-proof-init/bhtm-proof-init.sh`.
- Job receiver: `scripts/dracut/98bhtm-proof-init/bhtm-job-server.py`.
- Operator helper: `scripts/bhtm_proof_one_chunk.sh`.
- Output artifact default: `/tmp/bhtm-proof-generic.efi`.
- Current generic UKI built on `pir-hetzner` with the mirrored VPSBG
  `7.0.0-15-generic` kernel:
  - local copy: `/tmp/bhtm-proof-generic.efi`
  - remote copy: `pir-hetzner:/tmp/bhtm-proof-generic.efi`
  - UKI SHA256:
    `a40fd434718efa634c887ed9dd47c584d39499951d1466fc5ed399f8ca70a81b`
  - Linux/x86_64 `blockhash-to-muhash` SHA256:
    `6811c23193e9392367856ca6d2149b6f772857d199340d36282d8df1d70170e6`
  - cmdline:
    `rdinit=/sbin/bhtm-proof-init console=ttyS0,115200 console=tty1 quiet loglevel=3 bhtm.job_port=8080 bhtm.static_ip=87.120.8.198/32 bhtm.gateway=172.16.0.1 bhtm.dns=9.9.9.9`
- Chunk-0 runtime job prepared locally:
  - raw job: `/tmp/bhtm-mainnet-000000-099999.job.json`
  - canonical job: `/tmp/bhtm-mainnet-000000-099999.canonical.job.json`
  - job SHA256:
    `c600e1e97228797494cc6b55d2559e640220d829d9459db787b8a191ce35782d`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-000000-099999/proof-generic`
- Chunk-1 runtime job prepared locally for stable-measurement follow-up:
  - raw job: `/tmp/bhtm-mainnet-100000-199999.job.json`
  - canonical job: `/tmp/bhtm-mainnet-100000-199999.canonical.job.json`
  - job SHA256:
    `2634e4e2599ecef17ce3887adab2e914d79c6a3e242ecc9261d60aaf7631d16d`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-100000-199999/proof-generic`
  - start anchor:
    height `99999`, block hash
    `000000000002d01c1fccc21636b607dfd930d31d01c3a62104612a1719011250`,
    start MuHash from `mainnet-100000-199999/manifest.json`.
- Local submit/verify helper:
  `scripts/submit-generic-proof-job.sh <canonical-job.json> [output-dir]`.
- HTTPS proof endpoint:
  `scripts/bhtm-proof-endpoint.py`, deployed to
  `pir-hetzner:/root/blockhash-to-muhash/proof-endpoint/bhtm_endpoint.py`.
  The deployed endpoint is v2/multi-chunk: it serves
  `/<chunk>/stream.bhtmbin.zst`, accepts proof artifact PUTs under
  `/<chunk>/proof-generic/`, and serves uploaded artifacts back over HTTPS.
  The old chunk-0-only endpoint is not sufficient for the generic UKI flow.
- The measured cmdline contains generic boot/network controls only, e.g.
  `rdinit=/sbin/bhtm-proof-init ... bhtm.job_port=8080
  bhtm.static_ip=87.120.8.198/32 bhtm.gateway=172.16.0.1 bhtm.dns=9.9.9.9`.
- Do not put per-chunk stream URLs, stream hashes, upload URLs, anchors, or
  expected MuHash values into the cmdline if the SEV-SNP MEASUREMENT must stay
  stable across chunks. Those belong in runtime `job.json`.

### Fixed generic UKI chunk proofs and streaming rebuild (2026-06-25)

The corrected raw-report UKI has produced real proofs for
`mainnet-000000-099999` and `mainnet-100000-199999`.

- UKI SHA256:
  `a40fd434718efa634c887ed9dd47c584d39499951d1466fc5ed399f8ca70a81b`
- Linux/x86_64 `blockhash-to-muhash` SHA256:
  `6811c23193e9392367856ca6d2149b6f772857d199340d36282d8df1d70170e6`
- Local proof directory:
  `/tmp/bhtm-proof-mainnet-000000-099999-generic-fixed-report`
- Remote proof archive:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/proof-generic-fixed-report-uki-a40fd434-20260625/`
- Updated remote manifest SHA256:
  `743593580e3ad61a324da00b0acc805c752858a75bec2240720bc53325e28948`
- job SHA256:
  `c600e1e97228797494cc6b55d2559e640220d829d9459db787b8a191ce35782d`
- attestation SHA256:
  `a6b45fb38c98a79a175bae879069a31301c9316cd2c605c1930e5278d7f5daab`
- SEV-SNP report SHA256:
  `c2727f95c3f36d069ecea435cd91efb26d6f62f0661764924847f0a48c338eef`
- MEASUREMENT:
  `9373e779635cecb22bfde72250e4e27c887758d4e7bbc0bdf31b4b2c33bf46964a1587b921c2002faff256dfd980fd7d`
- REPORT_DATA:
  `364d14da44b114f3928d513cce16a3f4e672aa66647d1c951fd371b368d678ea43067d413e1146a002d4dfcaf3526ce1ceb7329af808c4e3097b893786ab9372`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.
- AMD KDS URL uses the short non-zero chip ID
  `003642735DDC6E02`; the full zero-padded 64-byte chip ID returns 404 on
  this Turin endpoint.

Chunk 1 proof:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-100000-199999-generic-fixed-report`
- Remote proof archive:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-100000-199999/proof-generic-fixed-report-uki-a40fd434-20260625/`
- Updated remote manifest SHA256:
  `b6e5b7b3832e530c92f3a9951a53bdefdcee8451253fc4e2723e1bc061a490a0`
- job SHA256:
  `2634e4e2599ecef17ce3887adab2e914d79c6a3e242ecc9261d60aaf7631d16d`
- attestation SHA256:
  `256d2e168d58e8844c8a6d8f7ed5757a0836f01e6489c6c41a02d684b4ca848f`
- SEV-SNP report SHA256:
  `1bf270eadfd8fb576cf563dcd81b58baaeb0e9d9a620ae8da4975ba976b7c893`
- MEASUREMENT:
  `9373e779635cecb22bfde72250e4e27c887758d4e7bbc0bdf31b4b2c33bf46964a1587b921c2002faff256dfd980fd7d`
- REPORT_DATA:
  `f99274892cdb334ee4930dbcdd97fe932a12c647d4bb1b6520e0d19d692a05f03b7a23ee6e2e07b70649a53ee4c1265a9c649861670a8ac960b5b94fbd495b65`
- End Core MuHash:
  `e1e783e9ac237240789cb97cf37b8e108c038b44b07e9c58768233bfb1a80f06`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 0 and chunk 1 reported the same old fixed-image MEASUREMENT while binding
different `job_sha256` values. Those fixed-image proof directories have now
been moved aside; live `proof-generic/` is reserved for streaming-UKI proofs.

Attempting chunk 2 with the same `a40fd...` UKI exposed an input-storage
problem: the init script wrote both `/tmp/chunk.bin.zst` and `/tmp/chunk.bin`
to tmpfs. `mainnet-200000-299999` needs about 13 GB compressed plus 28 GB
decompressed, and later chunks are much larger. A new streaming UKI was built:

- Local upload artifact:
  `/tmp/bhtm-proof-generic-streaming.efi`
- Conventional local copy:
  `/tmp/bhtm-proof-generic.efi`
- Remote build artifact:
  `pir-hetzner:/tmp/bhtm-proof-generic.efi`
- UKI SHA256:
  `88054be8b91143209a753a7d5ac887e966a3829ea85d8881f5fc6b37f1b90700`
- Linux/x86_64 `blockhash-to-muhash` SHA256:
  `6811c23193e9392367856ca6d2149b6f772857d199340d36282d8df1d70170e6`
- Init path now streams:
  `curl | tee(hash fifo) | zstd -dc | blockhash-to-muhash job-teed -`.
- Proof upload still happens only after the compressed stream SHA256 matches
  the runtime job.

Chunk 0 streaming-UKI redo succeeded with the same UKI used by chunks 2+:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-000000-099999-generic-streaming-redo`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/proof-generic/`
- Updated remote manifest SHA256:
  `991df594eecbb517803f1d48ad978453597d8aea92f1b7f49abd110551326ea4`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-000000-099999.proof-generic-and-manifest.streaming-redo.tar.zst`
- Backup bundle SHA256:
  `e6eff272c2afe509ad1cfe2f3e495c1125c8d6f85a39c7ee2eca210b3768e8a2`
- job SHA256:
  `c600e1e97228797494cc6b55d2559e640220d829d9459db787b8a191ce35782d`
- SEV-SNP report SHA256:
  `3a4e0b87b2f8b3d002c0efa3f384f42106310242141b9ec9388e1d9bcb0d572f`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- Tree root:
  `4ae1b5fcc5eebedefbbd9b8a28282c524601ee48ac74814b1b48e3f3bedec0ce`
- End Core MuHash:
  `f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 1 streaming-UKI redo also succeeded with the same UKI used by chunks 2+:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-100000-199999-generic-streaming-redo`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-100000-199999/proof-generic/`
- Updated remote manifest SHA256:
  `57dc1f88464adbc39b6c4edcc48a9fa899a3e3193dfa3d814edd92e38ab11023`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-100000-199999.proof-generic-and-manifest.streaming-redo.tar.zst`
- Backup bundle SHA256:
  `f7b9dcaf024d1cb2c3475d056e125f0d7f2dd70242627400cbc03eae017ab1e5`
- job SHA256:
  `2634e4e2599ecef17ce3887adab2e914d79c6a3e242ecc9261d60aaf7631d16d`
- attestation SHA256:
  `256d2e168d58e8844c8a6d8f7ed5757a0836f01e6489c6c41a02d684b4ca848f`
- SEV-SNP report SHA256:
  `d1d9686084eee3a978897ba630f6427160346db20cd59fce72c1dd838a259d0e`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `f99274892cdb334ee4930dbcdd97fe932a12c647d4bb1b6520e0d19d692a05f03b7a23ee6e2e07b70649a53ee4c1265a9c649861670a8ac960b5b94fbd495b65`
- Tree root:
  `de5bc1bc10cf94557f8916e693cc628b4d5b40d46b326d9bb772414fc3b6ec01`
- End Core MuHash:
  `e1e783e9ac237240789cb97cf37b8e108c038b44b07e9c58768233bfb1a80f06`
- TEE elapsed:
  `94.205886543s`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 2 streaming proof succeeded with this new UKI:

- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-200000-299999/proof-generic/`
- Updated remote manifest SHA256:
  `9de1786ee47e879f3577d651d68938d2eafb52b59ccdaef76e96f5d6d6b4fcc1`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-200000-299999.proof-generic-and-manifest.tar.zst`
- Backup bundle SHA256:
  `5acaf791ae53933678f6d9d4ea3a0de8e610d391169c9c08108a31fbf1b1e480`
- job SHA256:
  `928dacf132aa69bcd3706423e2660457739362ac1432274f79b9d21e89e9118a`
- attestation SHA256:
  `e07aef592d575baad72367697cde61ba0578d0b17a49674beef2b802748c2896`
- SEV-SNP report SHA256:
  `f1218cc8395ccb48ecbe48ace9cde967db25e7e00d46401a99b80925a7b35c74`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `f6022c57b13b6d6a717c60911d921933bdfd259d2790b0e8789f445f96dea99c16570d27d2664b910a9ebf75c4871b3a2a35551dbbbff7809b5e72e262359a46`
- Tree root:
  `e2f9dfccdcc4058d36e3a045fd2c084107315ae6686cee4da4d7e0c1da37415f`
- End Core MuHash:
  `c8df37777612a7d513ffce0d17921cd0bc43e375f40f5f374c829dfbbd6cd3a0`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 3 streaming proof also succeeded with the same UKI:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-300000-399999-generic-streaming`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-300000-399999/proof-generic/`
- Updated remote manifest SHA256:
  `494b71c969ac85d1d99304a01622dca4b85d22a930a4efd4ecc2ad605a7ac927`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-300000-399999.proof-generic-and-manifest.tar.zst`
- Backup bundle SHA256:
  `24a8da3d25d2b58633d44d5bcbf04ad8d00d14843966a443c0a1784010b32c25`
- job SHA256:
  `7b2e3c542f08b633e6f9b2a3959277ad4841230ca660afd17ea630ae02cafb05`
- attestation SHA256:
  `242569a2414e535b89cfc71a82535a7ce3da2ba77e69dba788709e2931ecd5f9`
- SEV-SNP report SHA256:
  `ae1dcdaddfc8197389f0d441511163552ee36b0d67056709c68e4fac69437fcd`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `9ad1541fe0e17955acfbc72ee74583d30a5870e70fe73af49db09435a993f97473ae0fdc5b3927c81450e6f9985d80527f6082a28b7dda267efbfbd16c57faf8`
- Tree root:
  `31516b07223100f319690f1b057227960549638cd879a311822f659c62fdeb2b`
- End Core MuHash:
  `18cf1650a245e0979bfeb64495d04e39fa1dd1fbefca29aeb3bdecae7631ade1`
- TEE elapsed:
  `3651.778807626s`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 4 streaming proof also succeeded with the same UKI:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-400000-499999-generic-streaming`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-400000-499999/proof-generic/`
- Updated remote manifest SHA256:
  `26cf5f45f9baae485fcebd450c7a2446080a1e5a93f252ac3347d389452fbc10`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-400000-499999.proof-generic-and-manifest.tar.zst`
- Backup bundle SHA256:
  `977e645e36eb926acd71fb8238ca869a8188f33f2d577dbf9b7fe232e142540f`
- job SHA256:
  `2cdaccd7ec96c87f828b60eaa0ccbe606be76165114a6289b74fc82de8f16f7f`
- attestation SHA256:
  `57b9f69a37021c785748b73bad11fd761ee2bdd8e64a7058575c8cc73caeea58`
- SEV-SNP report SHA256:
  `be4b189af00a0eb9c3b240b85c8d78ed9d0e211c3ad0ea8ba462df8227bdf40f`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `f6969f606c149536ce60b3c87524e3755b6a39f26cd5714b933624ed316bf5243ac21ca9df13e7a7ddb02e4fb1f686ffc9510281d41ef0cb04b4b9949683b5d0`
- Tree root:
  `38a0d50733e5209dcc9de2ab8e27551b3edf975e8dd96fdbc7eac52cf09a054c`
- End Core MuHash:
  `171e08e48bad8674528fb8d1a6fd9de42b4cfcdf78d8caf6558016fb9cb2a4ed`
- TEE elapsed:
  `3383.901518338s`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 5 streaming proof also succeeded with the same UKI:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-500000-599999-generic-streaming`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-500000-599999/proof-generic/`
- Updated remote manifest SHA256:
  `1ef3d7659a9810317d81ce5d95e98f177c5b345feca7c1ca5174ce5a356c2014`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-500000-599999.proof-generic-and-manifest.tar.zst`
- Backup bundle SHA256:
  `47cc73801cdc8e85cb056c2ae8c87ca96983b6d25376874cc620f83185e069f7`
- job SHA256:
  `37665da267352a4b5569365a1410feb2871959ccd9a952f7f9dccce82408afb1`
- attestation SHA256:
  `634438d1b9e9eec5af63ae440b21463cb2aee3ca20b54750910f08eb0677a544`
- SEV-SNP report SHA256:
  `52955a379d9657794a7a4e31fa4621ccd6ccc83edc220241801ab79b8a49af1c`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `05f94c427ae8c84513dadf70ec0c7d6961bfe442ea519c670fb4268470cfcd510d7e7ebf603cfd232fc22a43c4961736e984e30f99eab3e24867aa471da36e82`
- Tree root:
  `f6abf2e16557a6ef9dd7373e955360ac1a1293f919cab65d2fee96224cabb08a`
- End Core MuHash:
  `53882696420706cc51cedb6082413569196f667d9d4b8d87d6dc2a9abd6ac4ed`
- TEE elapsed:
  `7720.602391566s`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 6 streaming proof also succeeded with the same UKI:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-600000-699999-generic-streaming`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-600000-699999/proof-generic/`
- Updated remote manifest SHA256:
  `bd83c3605af1868eae285d023b65f95ba4956c165a196611864f004731448396`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-600000-699999.proof-generic-and-manifest.tar.zst`
- Backup bundle SHA256:
  `6c35f4ec5166f93122e5d1e391e24d781abf37edd7351aebc0d14908f58f16a9`
- job SHA256:
  `923ce6f2949b73d5397c9964b15260a60eb1047da2187592ed7ca2ae31c838a6`
- attestation SHA256:
  `3a6a0dc291c9410ac9f9892adcab8f793b7e3b5134ed6cbecead527fcf256abe`
- SEV-SNP report SHA256:
  `57161965257eef033abcf09a30898d8a4c76a0e6dab434d197cfe729a0894bfa`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `bafa78086189910f66cd979ae1b8ab0680c185707520fb083c6355ae38d04e2f2eb927989709b65de11ea01dd428b47f7c0003854ee15a74074cef2c464b5b00`
- Tree root:
  `a94b44aa8abbd882637ddc8fc9b798d0ae098e1422c2a9d50c10ce11aaf23a12`
- End Core MuHash:
  `5382e786ee8a1cc4e92d59030295e9cc0d3c0e3aea9c66e2fb9ee7cdae9d6f55`
- TEE elapsed:
  `5963.400253969s`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 7 streaming proof also succeeded with the same UKI:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-700000-799999-generic-streaming`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-700000-799999/proof-generic/`
- Updated remote manifest SHA256:
  `a954787a52d1612b300767d05ce7b3208e03609b103b0b3d185dadbbd49ba604`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-700000-799999.proof-generic-and-manifest.tar.zst`
- Backup bundle SHA256:
  `6557cdd3ad73423d6b2e810493c00bb7ff64f5439c5c53d4f4b8c420dec93c2b`
- job SHA256:
  `faeb2560210b800cf4d1fb1c949e2d3b7f29e33cd70666f0700f0d5c4b47269b`
- attestation SHA256:
  `00ce6b5527b2399fd8ef5c92c2f982859768919daa9e516ca81f2bd542dad7ec`
- SEV-SNP report SHA256:
  `00caa62f032dd8f3222366f163c4e56f17e75474bb5e917e9fb40a7076319afa`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `09ffb64c1947fdb8782f60823027c708b004d19ebb3963bc847a48b5e196ee0c1b4b01b142feab33e71655ddeba8f5e4e7a0097388fed0a139f699cdeed0fd21`
- Tree root:
  `ac3762584df067f34e46fbe5ad8ddc6ec3ead3d7073bc2d5f73fc06fbdd3411b`
- End Core MuHash:
  `9226d85add58fe2635c468013944cdefb8fb8fae995888ea76621ff05ffc1c21`
- TEE elapsed:
  `6104.719346204s`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Chunk 8 streaming proof also succeeded with the same UKI:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-800000-899999-generic-streaming`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-800000-899999/proof-generic/`
- Updated remote manifest SHA256:
  `e1fb14ff7072418555984c1cef2ea387bbf91ae372ca5269211ed377181dd69e`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-800000-899999.proof-generic-and-manifest.tar.zst`
- Backup bundle SHA256:
  `23375569a16eacaf338f6c04bf6f6f37ff5c03448634757ad7b450cb2b205687`
- job SHA256:
  `804ac08a819709691475ca6f6c12bb499f4349f3bb189eb3effadec2ec7d8ae1`
- attestation SHA256:
  `b7b13d9061ed3b9a0766888a9ecbae977bb3ba1a849143b29500f873fe08d7f1`
- SEV-SNP report SHA256:
  `e1dadc817aff72ba1156c4ed3d8428bf5b46603326c1fb9e83c4a1e98f287a61`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `30090fa7cad97891327221bb8d60bd71e31ae9005564f3376bd0975728e8670994701e2e4138315878aa4c5b9792751e51ca488c6f2537587c7ecfc332d31b7b`
- Tree root:
  `e3d61a7e18dc74ea798470917e40d8e578de946fe32e13022a8d986236dcc14b`
- End Core MuHash:
  `1ec8dedf1473bd17f421c1dd7c78d6ba61747ac3161c14a27322b528bb032ca9`
- TEE elapsed:
  `5621.341159177s`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Partial current-head chunk `mainnet-900000-954920` streaming proof also
succeeded with the same UKI:

- Local proof directory:
  `/tmp/bhtm-proof-mainnet-900000-954920-generic-streaming`
- Remote proof directory:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-900000-954920/proof-generic/`
- Updated remote manifest SHA256:
  `11286a0ee29edb10c5f9b8383dece85fad8183136f41035796eb2a8fa1b95f6f`
- Backup bundle:
  `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-900000-954920.proof-generic-and-manifest.tar.zst`
- Backup bundle SHA256:
  `f9f30ff8248a800ff7ade3985681d25f6356a7d24359ed68e14caa1a1941580b`
- job SHA256:
  `19e03b71ea9150b1c64d4a0069469384420a22dd4bcffd0ae1300a7121331e52`
- attestation SHA256:
  `4c360e56c50f0d38747682932e35d9f54ce9992893bf7b61bbe62d16f37158ed`
- SEV-SNP report SHA256:
  `383523d838d36d8aefedf57fc938b209c6d1b9cd24d9f23c5cb68b0ea7da03c6`
- MEASUREMENT:
  `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
- REPORT_DATA:
  `6c8f972108a26b2d17ec1e5741c9134449db7fb1dca6a97d8d9b8a51ec1557d75fc905bdf67ecdf9e102908498eaa033a82a78f1910806d66d619412074836b1`
- Tree root:
  `babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6`
- End Core MuHash:
  `c27e1fafa9dd154cfd16a78429477181febbecf49b3d49016c462b0764429470`
- TEE elapsed:
  `2303.465806605s`
- `verify-bhtm-quote`: `status=ok`, `attestation_version=2`,
  `report_data_match=ok`.
- AMD VCEK chain/report-signature verification:
  `amd_chain_and_report_signature=ok`.

Uniform-image cleanup is complete: chunk 0, chunk 1, chunks 2 through 8, and the
partial head chunk now share the same streaming UKI measurement. The earlier
fixed-raw-report generic proofs for chunk 0 and chunk 1 are preserved in their
archive directories.

Prepared final-partial submit command, now completed:

- canonical job:
  `/tmp/bhtm-mainnet-900000-954920.canonical.job.json`
- remote prepared job:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-900000-954920/proof-generic/job.prepared.json`
- submit command:
  `SUBMIT_TIMEOUT=7200 PROOF_TIMEOUT=43200 BHTM_BINARY=./target/release/blockhash-to-muhash scripts/submit-generic-proof-job.sh /tmp/bhtm-mainnet-900000-954920.canonical.job.json /tmp/bhtm-proof-mainnet-900000-954920-generic-streaming`

Earlier chunk 8 submit command:

- canonical job:
  `/tmp/bhtm-mainnet-800000-899999.canonical.job.json`
- remote prepared job:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-800000-899999/proof-generic/job.prepared.json`
- job SHA256:
  `804ac08a819709691475ca6f6c12bb499f4349f3bb189eb3effadec2ec7d8ae1`
- expected end Core MuHash:
  `1ec8dedf1473bd17f421c1dd7c78d6ba61747ac3161c14a27322b528bb032ca9`
- submitted via direct guest PUT:
  `curl -X PUT --data-binary @/tmp/bhtm-mainnet-800000-899999.canonical.job.json http://87.120.8.198:8080/job`
- accepted response:
  `ok`
- first Hetzner stream check:
  `2026-06-25T16:04:30Z stream bytes_sent=1022947864 pct=1.258`
- fallback submit command if the run must be repeated after confirming no proof
  artifacts landed:
  `SUBMIT_TIMEOUT=7200 PROOF_TIMEOUT=43200 BHTM_BINARY=./target/release/blockhash-to-muhash scripts/submit-generic-proof-job.sh /tmp/bhtm-mainnet-800000-899999.canonical.job.json /tmp/bhtm-proof-mainnet-800000-899999-generic-streaming`

Earlier chunk 7 submit command:

- canonical job:
  `/tmp/bhtm-mainnet-700000-799999.canonical.job.json`
- remote prepared job:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-700000-799999/proof-generic/job.prepared.json`
- job SHA256:
  `faeb2560210b800cf4d1fb1c949e2d3b7f29e33cd70666f0700f0d5c4b47269b`
- expected end Core MuHash:
  `9226d85add58fe2635c468013944cdefb8fb8fae995888ea76621ff05ffc1c21`
- submit command:
  `SUBMIT_TIMEOUT=7200 PROOF_TIMEOUT=43200 BHTM_BINARY=./target/release/blockhash-to-muhash scripts/submit-generic-proof-job.sh /tmp/bhtm-mainnet-700000-799999.canonical.job.json /tmp/bhtm-proof-mainnet-700000-799999-generic-streaming`

Earlier chunk 6 submit command:

- canonical job:
  `/tmp/bhtm-mainnet-600000-699999.canonical.job.json`
- remote prepared job:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-600000-699999/proof-generic/job.prepared.json`
- job SHA256:
  `923ce6f2949b73d5397c9964b15260a60eb1047da2187592ed7ca2ae31c838a6`
- expected end Core MuHash:
  `5382e786ee8a1cc4e92d59030295e9cc0d3c0e3aea9c66e2fb9ee7cdae9d6f55`
- submit command:
  `SUBMIT_TIMEOUT=7200 PROOF_TIMEOUT=43200 BHTM_BINARY=./target/release/blockhash-to-muhash scripts/submit-generic-proof-job.sh /tmp/bhtm-mainnet-600000-699999.canonical.job.json /tmp/bhtm-proof-mainnet-600000-699999-generic-streaming`

Earlier chunk 5 submit command:

- canonical job:
  `/tmp/bhtm-mainnet-500000-599999.canonical.job.json`
- remote prepared job:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-500000-599999/proof-generic/job.prepared.json`
- job SHA256:
  `37665da267352a4b5569365a1410feb2871959ccd9a952f7f9dccce82408afb1`
- expected end Core MuHash:
  `53882696420706cc51cedb6082413569196f667d9d4b8d87d6dc2a9abd6ac4ed`
- submit command:
  `SUBMIT_TIMEOUT=7200 PROOF_TIMEOUT=43200 BHTM_BINARY=./target/release/blockhash-to-muhash scripts/submit-generic-proof-job.sh /tmp/bhtm-mainnet-500000-599999.canonical.job.json /tmp/bhtm-proof-mainnet-500000-599999-generic-streaming`

Chunk 0 per-chunk proof already succeeded before this generic refactor:

- Proof directory copied locally:
  `/tmp/bhtm-proof-mainnet-000000-099999-proof/`.
- `verify-bhtm-quote`: `status=ok`, `report_data_match=ok`.
- attestation sha256:
  `657b823f80dd031134534331197192bdd265900155b31c3262f62530f930285c`
- SEV-SNP report sha256:
  `13d9b45edd4d608b0dac8c450b767e0eb5fa68b889647c9b5c31c09783bbca17`
- MEASUREMENT:
  `257bb9abc34a77f54e0568f2ec199eeb32becfae8cb3e253f2a1b96911b70ff668c211995fbd6a72945e4d573970d3f3`
- end height: `99999`
- end Core MuHash:
  `f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4`
- tree root:
  `4ae1b5fcc5eebedefbbd9b8a28282c524601ee48ac74814b1b48e3f3bedec0ce`

The previous Linux/x86_64 binary SHA
`8651781558ea46117fceb83eaeca0bba09057b44ba7852e1b05fbe5e5b842283`
is stale after the generic job/v2 and raw-report source changes. Use the
rebuilt Linux SHA
`6811c23193e9392367856ca6d2149b6f772857d199340d36282d8df1d70170e6`
for the generic UKI above. The current fixed generic UKI has already produced
real chunk-0 and chunk-1 proofs with matching MEASUREMENT.

Tests after the generic-job changes:

- `cargo test`: 67 passing locally
  (51 lib + 4 CLI unit + 7 integration + 5 `sev_snp_smoke`).

### Known stub from a prior SEV-SNP run

`/home/pir/data/attestations/delta_940611_948454_sev_snp/build-evidence.sev-snp-report.bin`
on Hetzner is **not** a real AMD-signed SNP report. The REPORT_DATA
field at offset 0x50 does
not match the `build-evidence.report-data` value
`a8e04cc4…0a79e02fc12a…fbda`. Its raw-report fields do not match a
valid AMD SEV-SNP report layout, and the SIGNATURE field is malformed.

This is the PIR-database attested builder's output (different target
from the chunk archive); it's referenced here only because both
pipelines share the SEV-SNP host. The new `verify-bhtm-quote` tool will
correctly reject this stub if asked, since its REPORT_DATA field
doesn't match the file's report-data value.
