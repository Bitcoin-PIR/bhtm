# AGENTS.md - Block Hash -> MuHash Binding

The user prefers Codex to continue running until finishing the task.

## Quick Start

```bash
cd /Users/cusgadmin/bitcoin-pir/blockhash-to-muhash
cargo test          # 67 tests, all pass (51 lib + 4 CLI + 7 integration + 5 sev_snp_smoke)
cargo build --release
```

Run the slim builder over prepared stripped block deltas:

```bash
target/release/blockhash-to-muhash bin stripped-deltas.bin
prepare-deltas ... | target/release/blockhash-to-muhash bin -
target/release/blockhash-to-muhash bin stripped-deltas.bin \
  --expect-end-core-muhash <core-display-muhash> \
  --attestation-out proof/attestation.bin \
  --report-data-out proof/report-data.bin \
  --stats-json-out proof/stats.json
```

Build a SEV-SNP attestation triple directly from CLI (Unix-only,
requires `/dev/sev-guest`):

```bash
target/release/blockhash-to-muhash bin-teed stripped-deltas.bin \
  --expect-end-core-muhash <core-display-muhash>
# Writes ./proof/{attestation.bin, report-data.bin, sev-snp-report.bin, stats.json}
```

For the generic reusable proof-runner UKI, chunk-specific data is
supplied at runtime as `job.json` and bound into attestation v2 by
`job_sha256`:

```bash
target/release/blockhash-to-muhash canonical-job job.json > proof/job.json
target/release/blockhash-to-muhash job-env proof/job.json
target/release/blockhash-to-muhash job-teed proof/job.json stripped-deltas.bin
# Writes ./proof/{job.json,attestation.bin,report-data.bin,sev-snp-report.bin,stats.json}
```

Verify the triple locally (does NOT verify the AMD signature; pair with
`pir-attest-verify`):

```bash
target/release/blockhash-to-muhash verify-bhtm-quote \
  proof/attestation.bin proof/report-data.bin proof/sev-snp-report.bin
```

Operator-pinned release binary SHA256 is **platform-specific**. Rebuild with
`cargo build --release` against the pinned `rust-toolchain.toml`; Linux/x86_64
(VPSBG build host) is what matters for the SEV-SNP UKI, while macOS/arm64
(development host) is a separate pin. The Linux pin below was rebuilt on
`pir-hetzner` against the mirrored VPSBG kernel/tooling with Rust 1.94.1.

```text
Linux x86_64, Rust 1.94.1: 6811c23193e9392367856ca6d2149b6f772857d199340d36282d8df1d70170e6
macOS arm64,  Rust 1.94.1: 15ae5fb44b974e3995119d45e254f7e990fa819c857eba785dae73b8339decb2
```

When building the VPSBG UKI, you must hit the **Linux x86_64** pin.
The `rust-toolchain.toml` file in this repo pins the toolchain
(`1.94.1`), and the actual binary SHA is reproducible only across
operators running the same OS+arch.

The SEV-SNP MRTD chain pins the binary baked into the reusable generic
UKI (`scripts/build_bhtm_proof_uki.sh`). The toolchain pin
in `rust-toolchain.toml` is `1.94.1`, matching
`BitcoinPIR/rust-toolchain.toml`.

Current streaming generic UKI artifact for chunk 2+:

```text
Local:  /tmp/bhtm-proof-generic.efi
Local:  /tmp/bhtm-proof-generic-streaming.efi
Remote: pir-hetzner:/tmp/bhtm-proof-generic.efi
SHA256: 88054be8b91143209a753a7d5ac887e966a3829ea85d8881f5fc6b37f1b90700
Bytes:  295 MiB local display size
Cmdline: rdinit=/sbin/bhtm-proof-init console=ttyS0,115200 console=tty1 quiet loglevel=3 bhtm.job_port=8080 bhtm.static_ip=87.120.8.198/32 bhtm.gateway=172.16.0.1 bhtm.dns=9.9.9.9
```

The earlier fixed raw-report generic UKI used for chunk 0 and chunk 1 had
SHA256 `a40fd434718efa634c887ed9dd47c584d39499951d1466fc5ed399f8ca70a81b`.
It materialized both compressed and decompressed streams in `/tmp` and is
therefore unsuitable for chunk 2+.

The streaming UKI keeps the same job/attestation v2 format but runs:
`curl | tee(hash fifo) | zstd -dc | blockhash-to-muhash job-teed -`.
It uploads proofs only after the compressed stream SHA256 matches the
runtime job. This avoids requiring tmpfs space for full chunks.

Prepared generic proof jobs:

```text
mainnet-000000-099999:
  canonical job: /tmp/bhtm-mainnet-000000-099999.canonical.job.json
  job_sha256: c600e1e97228797494cc6b55d2559e640220d829d9459db787b8a191ce35782d
  upload_url: https://65-21-91-217.sslip.io/mainnet-000000-099999/proof-generic
mainnet-100000-199999:
  canonical job: /tmp/bhtm-mainnet-100000-199999.canonical.job.json
  job_sha256: 2634e4e2599ecef17ce3887adab2e914d79c6a3e242ecc9261d60aaf7631d16d
  upload_url: https://65-21-91-217.sslip.io/mainnet-100000-199999/proof-generic
mainnet-200000-299999:
  canonical job: /tmp/bhtm-mainnet-200000-299999.canonical.job.json
  job_sha256: 928dacf132aa69bcd3706423e2660457739362ac1432274f79b9d21e89e9118a
  upload_url: https://65-21-91-217.sslip.io/mainnet-200000-299999/proof-generic
mainnet-300000-399999:
  canonical job: /tmp/bhtm-mainnet-300000-399999.canonical.job.json
  job_sha256: 7b2e3c542f08b633e6f9b2a3959277ad4841230ca660afd17ea630ae02cafb05
  upload_url: https://65-21-91-217.sslip.io/mainnet-300000-399999/proof-generic
mainnet-400000-499999:
  canonical job: /tmp/bhtm-mainnet-400000-499999.canonical.job.json
  job_sha256: 2cdaccd7ec96c87f828b60eaa0ccbe606be76165114a6289b74fc82de8f16f7f
  upload_url: https://65-21-91-217.sslip.io/mainnet-400000-499999/proof-generic
mainnet-500000-599999:
  canonical job: /tmp/bhtm-mainnet-500000-599999.canonical.job.json
  job_sha256: 37665da267352a4b5569365a1410feb2871959ccd9a952f7f9dccce82408afb1
  upload_url: https://65-21-91-217.sslip.io/mainnet-500000-599999/proof-generic
mainnet-600000-699999:
  canonical job: /tmp/bhtm-mainnet-600000-699999.canonical.job.json
  job_sha256: 923ce6f2949b73d5397c9964b15260a60eb1047da2187592ed7ca2ae31c838a6
  upload_url: https://65-21-91-217.sslip.io/mainnet-600000-699999/proof-generic
mainnet-700000-799999:
  canonical job: /tmp/bhtm-mainnet-700000-799999.canonical.job.json
  job_sha256: faeb2560210b800cf4d1fb1c949e2d3b7f29e33cd70666f0700f0d5c4b47269b
  upload_url: https://65-21-91-217.sslip.io/mainnet-700000-799999/proof-generic
mainnet-800000-899999:
  canonical job: /tmp/bhtm-mainnet-800000-899999.canonical.job.json
  job_sha256: 804ac08a819709691475ca6f6c12bb499f4349f3bb189eb3effadec2ec7d8ae1
  upload_url: https://65-21-91-217.sslip.io/mainnet-800000-899999/proof-generic
mainnet-900000-954920:
  canonical job: /tmp/bhtm-mainnet-900000-954920.canonical.job.json
  job_sha256: 19e03b71ea9150b1c64d4a0069469384420a22dd4bcffd0ae1300a7121331e52
  upload_url: https://65-21-91-217.sslip.io/mainnet-900000-954920/proof-generic
```

Fixed raw-report generic proof status:

```text
mainnet-000000-099999:
  proof local: /tmp/bhtm-proof-mainnet-000000-099999-generic-fixed-report
  proof remote archive: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/proof-generic-fixed-report-uki-a40fd434-20260625/
  manifest_sha256: 743593580e3ad61a324da00b0acc805c752858a75bec2240720bc53325e28948
  job_sha256: c600e1e97228797494cc6b55d2559e640220d829d9459db787b8a191ce35782d
  attestation_sha256: a6b45fb38c98a79a175bae879069a31301c9316cd2c605c1930e5278d7f5daab
  sev_snp_report_sha256: c2727f95c3f36d069ecea435cd91efb26d6f62f0661764924847f0a48c338eef
  measurement: 9373e779635cecb22bfde72250e4e27c887758d4e7bbc0bdf31b4b2c33bf46964a1587b921c2002faff256dfd980fd7d
  report_data: 364d14da44b114f3928d513cce16a3f4e672aa66647d1c951fd371b368d678ea43067d413e1146a002d4dfcaf3526ce1ceb7329af808c4e3097b893786ab9372
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-100000-199999:
  proof local: /tmp/bhtm-proof-mainnet-100000-199999-generic-fixed-report
  proof remote archive: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-100000-199999/proof-generic-fixed-report-uki-a40fd434-20260625/
  manifest_sha256: b6e5b7b3832e530c92f3a9951a53bdefdcee8451253fc4e2723e1bc061a490a0
  job_sha256: 2634e4e2599ecef17ce3887adab2e914d79c6a3e242ecc9261d60aaf7631d16d
  attestation_sha256: 256d2e168d58e8844c8a6d8f7ed5757a0836f01e6489c6c41a02d684b4ca848f
  sev_snp_report_sha256: 1bf270eadfd8fb576cf563dcd81b58baaeb0e9d9a620ae8da4975ba976b7c893
  measurement: 9373e779635cecb22bfde72250e4e27c887758d4e7bbc0bdf31b4b2c33bf46964a1587b921c2002faff256dfd980fd7d
  report_data: f99274892cdb334ee4930dbcdd97fe932a12c647d4bb1b6520e0d19d692a05f03b7a23ee6e2e07b70649a53ee4c1265a9c649861670a8ac960b5b94fbd495b65
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
```

Both proofs use the same MEASUREMENT while binding different runtime
`job_sha256` values. This confirms the generic UKI image is stable across
chunks and per-chunk details are bound through attestation v2.

Streaming generic proof status:

```text
mainnet-000000-099999:
  proof local: /tmp/bhtm-proof-mainnet-000000-099999-generic-streaming-redo
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/proof-generic/
  manifest_sha256: 991df594eecbb517803f1d48ad978453597d8aea92f1b7f49abd110551326ea4
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-000000-099999.proof-generic-and-manifest.streaming-redo.tar.zst
  backup_sha256: e6eff272c2afe509ad1cfe2f3e495c1125c8d6f85a39c7ee2eca210b3768e8a2
  job_sha256: c600e1e97228797494cc6b55d2559e640220d829d9459db787b8a191ce35782d
  attestation_sha256: a6b45fb38c98a79a175bae879069a31301c9316cd2c605c1930e5278d7f5daab
  sev_snp_report_sha256: 3a4e0b87b2f8b3d002c0efa3f384f42106310242141b9ec9388e1d9bcb0d572f
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: 364d14da44b114f3928d513cce16a3f4e672aa66647d1c951fd371b368d678ea43067d413e1146a002d4dfcaf3526ce1ceb7329af808c4e3097b893786ab9372
  tree_root: 4ae1b5fcc5eebedefbbd9b8a28282c524601ee48ac74814b1b48e3f3bedec0ce
  end_core_muhash: f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4
  elapsed_secs: 3.787123625
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-100000-199999:
  proof local: /tmp/bhtm-proof-mainnet-100000-199999-generic-streaming-redo
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-100000-199999/proof-generic/
  manifest_sha256: 57dc1f88464adbc39b6c4edcc48a9fa899a3e3193dfa3d814edd92e38ab11023
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-100000-199999.proof-generic-and-manifest.streaming-redo.tar.zst
  backup_sha256: f7b9dcaf024d1cb2c3475d056e125f0d7f2dd70242627400cbc03eae017ab1e5
  job_sha256: 2634e4e2599ecef17ce3887adab2e914d79c6a3e242ecc9261d60aaf7631d16d
  attestation_sha256: 256d2e168d58e8844c8a6d8f7ed5757a0836f01e6489c6c41a02d684b4ca848f
  sev_snp_report_sha256: d1d9686084eee3a978897ba630f6427160346db20cd59fce72c1dd838a259d0e
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: f99274892cdb334ee4930dbcdd97fe932a12c647d4bb1b6520e0d19d692a05f03b7a23ee6e2e07b70649a53ee4c1265a9c649861670a8ac960b5b94fbd495b65
  tree_root: de5bc1bc10cf94557f8916e693cc628b4d5b40d46b326d9bb772414fc3b6ec01
  end_core_muhash: e1e783e9ac237240789cb97cf37b8e108c038b44b07e9c58768233bfb1a80f06
  elapsed_secs: 94.205886543
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-200000-299999:
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-200000-299999/proof-generic/
  manifest_sha256: 9de1786ee47e879f3577d651d68938d2eafb52b59ccdaef76e96f5d6d6b4fcc1
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-200000-299999.proof-generic-and-manifest.tar.zst
  backup_sha256: 5acaf791ae53933678f6d9d4ea3a0de8e610d391169c9c08108a31fbf1b1e480
  job_sha256: 928dacf132aa69bcd3706423e2660457739362ac1432274f79b9d21e89e9118a
  attestation_sha256: e07aef592d575baad72367697cde61ba0578d0b17a49674beef2b802748c2896
  sev_snp_report_sha256: f1218cc8395ccb48ecbe48ace9cde967db25e7e00d46401a99b80925a7b35c74
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: f6022c57b13b6d6a717c60911d921933bdfd259d2790b0e8789f445f96dea99c16570d27d2664b910a9ebf75c4871b3a2a35551dbbbff7809b5e72e262359a46
  tree_root: e2f9dfccdcc4058d36e3a045fd2c084107315ae6686cee4da4d7e0c1da37415f
  end_core_muhash: c8df37777612a7d513ffce0d17921cd0bc43e375f40f5f374c829dfbbd6cd3a0
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-300000-399999:
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-300000-399999/proof-generic/
  manifest_sha256: 494b71c969ac85d1d99304a01622dca4b85d22a930a4efd4ecc2ad605a7ac927
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-300000-399999.proof-generic-and-manifest.tar.zst
  backup_sha256: 24a8da3d25d2b58633d44d5bcbf04ad8d00d14843966a443c0a1784010b32c25
  job_sha256: 7b2e3c542f08b633e6f9b2a3959277ad4841230ca660afd17ea630ae02cafb05
  attestation_sha256: 242569a2414e535b89cfc71a82535a7ce3da2ba77e69dba788709e2931ecd5f9
  sev_snp_report_sha256: ae1dcdaddfc8197389f0d441511163552ee36b0d67056709c68e4fac69437fcd
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: 9ad1541fe0e17955acfbc72ee74583d30a5870e70fe73af49db09435a993f97473ae0fdc5b3927c81450e6f9985d80527f6082a28b7dda267efbfbd16c57faf8
  tree_root: 31516b07223100f319690f1b057227960549638cd879a311822f659c62fdeb2b
  end_core_muhash: 18cf1650a245e0979bfeb64495d04e39fa1dd1fbefca29aeb3bdecae7631ade1
  elapsed_secs: 3651.778807626
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-400000-499999:
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-400000-499999/proof-generic/
  manifest_sha256: 26cf5f45f9baae485fcebd450c7a2446080a1e5a93f252ac3347d389452fbc10
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-400000-499999.proof-generic-and-manifest.tar.zst
  backup_sha256: 977e645e36eb926acd71fb8238ca869a8188f33f2d577dbf9b7fe232e142540f
  job_sha256: 2cdaccd7ec96c87f828b60eaa0ccbe606be76165114a6289b74fc82de8f16f7f
  attestation_sha256: 57b9f69a37021c785748b73bad11fd761ee2bdd8e64a7058575c8cc73caeea58
  sev_snp_report_sha256: be4b189af00a0eb9c3b240b85c8d78ed9d0e211c3ad0ea8ba462df8227bdf40f
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: f6969f606c149536ce60b3c87524e3755b6a39f26cd5714b933624ed316bf5243ac21ca9df13e7a7ddb02e4fb1f686ffc9510281d41ef0cb04b4b9949683b5d0
  tree_root: 38a0d50733e5209dcc9de2ab8e27551b3edf975e8dd96fdbc7eac52cf09a054c
  end_core_muhash: 171e08e48bad8674528fb8d1a6fd9de42b4cfcdf78d8caf6558016fb9cb2a4ed
  elapsed_secs: 3383.901518338
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-500000-599999:
  proof local: /tmp/bhtm-proof-mainnet-500000-599999-generic-streaming
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-500000-599999/proof-generic/
  manifest_sha256: 1ef3d7659a9810317d81ce5d95e98f177c5b345feca7c1ca5174ce5a356c2014
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-500000-599999.proof-generic-and-manifest.tar.zst
  backup_sha256: 47cc73801cdc8e85cb056c2ae8c87ca96983b6d25376874cc620f83185e069f7
  job_sha256: 37665da267352a4b5569365a1410feb2871959ccd9a952f7f9dccce82408afb1
  attestation_sha256: 634438d1b9e9eec5af63ae440b21463cb2aee3ca20b54750910f08eb0677a544
  sev_snp_report_sha256: 52955a379d9657794a7a4e31fa4621ccd6ccc83edc220241801ab79b8a49af1c
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: 05f94c427ae8c84513dadf70ec0c7d6961bfe442ea519c670fb4268470cfcd510d7e7ebf603cfd232fc22a43c4961736e984e30f99eab3e24867aa471da36e82
  tree_root: f6abf2e16557a6ef9dd7373e955360ac1a1293f919cab65d2fee96224cabb08a
  end_core_muhash: 53882696420706cc51cedb6082413569196f667d9d4b8d87d6dc2a9abd6ac4ed
  elapsed_secs: 7720.602391566
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-600000-699999:
  proof local: /tmp/bhtm-proof-mainnet-600000-699999-generic-streaming
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-600000-699999/proof-generic/
  manifest_sha256: bd83c3605af1868eae285d023b65f95ba4956c165a196611864f004731448396
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-600000-699999.proof-generic-and-manifest.tar.zst
  backup_sha256: 6c35f4ec5166f93122e5d1e391e24d781abf37edd7351aebc0d14908f58f16a9
  job_sha256: 923ce6f2949b73d5397c9964b15260a60eb1047da2187592ed7ca2ae31c838a6
  attestation_sha256: 3a6a0dc291c9410ac9f9892adcab8f793b7e3b5134ed6cbecead527fcf256abe
  sev_snp_report_sha256: 57161965257eef033abcf09a30898d8a4c76a0e6dab434d197cfe729a0894bfa
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: bafa78086189910f66cd979ae1b8ab0680c185707520fb083c6355ae38d04e2f2eb927989709b65de11ea01dd428b47f7c0003854ee15a74074cef2c464b5b00
  tree_root: a94b44aa8abbd882637ddc8fc9b798d0ae098e1422c2a9d50c10ce11aaf23a12
  end_core_muhash: 5382e786ee8a1cc4e92d59030295e9cc0d3c0e3aea9c66e2fb9ee7cdae9d6f55
  elapsed_secs: 5963.400253969
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-700000-799999:
  proof local: /tmp/bhtm-proof-mainnet-700000-799999-generic-streaming
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-700000-799999/proof-generic/
  manifest_sha256: a954787a52d1612b300767d05ce7b3208e03609b103b0b3d185dadbbd49ba604
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-700000-799999.proof-generic-and-manifest.tar.zst
  backup_sha256: 6557cdd3ad73423d6b2e810493c00bb7ff64f5439c5c53d4f4b8c420dec93c2b
  job_sha256: faeb2560210b800cf4d1fb1c949e2d3b7f29e33cd70666f0700f0d5c4b47269b
  attestation_sha256: 00ce6b5527b2399fd8ef5c92c2f982859768919daa9e516ca81f2bd542dad7ec
  sev_snp_report_sha256: 00caa62f032dd8f3222366f163c4e56f17e75474bb5e917e9fb40a7076319afa
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: 09ffb64c1947fdb8782f60823027c708b004d19ebb3963bc847a48b5e196ee0c1b4b01b142feab33e71655ddeba8f5e4e7a0097388fed0a139f699cdeed0fd21
  tree_root: ac3762584df067f34e46fbe5ad8ddc6ec3ead3d7073bc2d5f73fc06fbdd3411b
  end_core_muhash: 9226d85add58fe2635c468013944cdefb8fb8fae995888ea76621ff05ffc1c21
  elapsed_secs: 6104.719346204
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-800000-899999:
  proof local: /tmp/bhtm-proof-mainnet-800000-899999-generic-streaming
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-800000-899999/proof-generic/
  manifest_sha256: e1fb14ff7072418555984c1cef2ea387bbf91ae372ca5269211ed377181dd69e
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-800000-899999.proof-generic-and-manifest.tar.zst
  backup_sha256: 23375569a16eacaf338f6c04bf6f6f37ff5c03448634757ad7b450cb2b205687
  job_sha256: 804ac08a819709691475ca6f6c12bb499f4349f3bb189eb3effadec2ec7d8ae1
  attestation_sha256: b7b13d9061ed3b9a0766888a9ecbae977bb3ba1a849143b29500f873fe08d7f1
  sev_snp_report_sha256: e1dadc817aff72ba1156c4ed3d8428bf5b46603326c1fb9e83c4a1e98f287a61
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: 30090fa7cad97891327221bb8d60bd71e31ae9005564f3376bd0975728e8670994701e2e4138315878aa4c5b9792751e51ca488c6f2537587c7ecfc332d31b7b
  tree_root: e3d61a7e18dc74ea798470917e40d8e578de946fe32e13022a8d986236dcc14b
  end_core_muhash: 1ec8dedf1473bd17f421c1dd7c78d6ba61747ac3161c14a27322b528bb032ca9
  elapsed_secs: 5621.341159177
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
mainnet-900000-954920:
  proof local: /tmp/bhtm-proof-mainnet-900000-954920-generic-streaming
  proof remote: pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-900000-954920/proof-generic/
  manifest_sha256: 11286a0ee29edb10c5f9b8383dece85fad8183136f41035796eb2a8fa1b95f6f
  backup: pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-900000-954920.proof-generic-and-manifest.tar.zst
  backup_sha256: f9f30ff8248a800ff7ade3985681d25f6356a7d24359ed68e14caa1a1941580b
  job_sha256: 19e03b71ea9150b1c64d4a0069469384420a22dd4bcffd0ae1300a7121331e52
  attestation_sha256: 4c360e56c50f0d38747682932e35d9f54ce9992893bf7b61bbe62d16f37158ed
  sev_snp_report_sha256: 383523d838d36d8aefedf57fc938b209c6d1b9cd24d9f23c5cb68b0ea7da03c6
  measurement: 652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
  report_data: 6c8f972108a26b2d17ec1e5741c9134449db7fb1dca6a97d8d9b8a51ec1557d75fc905bdf67ecdf9e102908498eaa033a82a78f1910806d66d619412074836b1
  tree_root: babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6
  end_core_muhash: c27e1fafa9dd154cfd16a78429477181febbecf49b3d49016c462b0764429470
  elapsed_secs: 2303.465806605
  verify-bhtm-quote: status=ok, report_data_match=ok
  AMD VCEK/signature: amd_chain_and_report_signature=ok
```

All archived chunks from `mainnet-000000-099999` through the partial
`mainnet-900000-954920` have now been proven with the same streaming generic UKI
measurement. The older fixed-raw-report proofs for chunk 0 and chunk 1 have
been moved aside into archive directories.

## Current Design

The MuHash builder is intentionally small. It receives externally prepared
`StrippedBlockDelta` records:

- `height`
- 80-byte `header`
- trusted `block_hash`
- raw non-witness `stripped_txs` from which txids are derived
- Bitcoin Core-compatible coin `additions`
- Bitcoin Core-compatible coin `removals`

For each record it verifies only:

1. `SHA256d(header) == block_hash`
2. `height == previous_height + 1`
3. `header.prev_block_hash == previous_block_hash`
4. `Merkle(txids) == header.merkle_root`

The production compact binary format must provide raw stripped transaction
bytes, not only txids. The builder computes `SHA256d(stripped_tx)` first. It
treats those bytes as opaque txid material and does not parse or validate the
transactions.

Then it applies the supplied additions/removals to MuHash and appends
`(height, block_hash, full_384_byte_muhash)` to the MuHash Merkle tree.

For non-genesis continuation, initialize the builder from a trusted anchor:
`start_height`, `start_block_hash`, and the normalized 384-byte `start_muhash`
value. The 32-byte commitment helper is not the run endpoint format.

## Explicit Non-Goals

The builder does not:

- read raw `blk*.dat` files
- call Bitcoin Core RPC
- maintain a UTXO set or in-memory `HashMap`
- validate proof of work or difficulty
- validate scripts, signatures, witness, coinbase maturity, or subsidy
- reconstruct spent coin metadata from transaction inputs
- implement BIP30, duplicate txid, genesis, or unspendable-output filtering

Those rules must be handled by the data source / delta extractor. In particular,
historical duplicate txids mean the extractor must follow Bitcoin Core's
coinstats/undo handling and emit exactly the coins Core would add and remove.

## Architecture

```text
src/lib.rs         pub mod: job, num3072, muhash, merkle_tree, stream,
                            sev_snp_guest (Unix), verify_quote
src/main.rs        CLI: bin <path|->, bin-teed <path|->,
                          canonical-job <job.json>, job-env <job.json>,
                          job <job.json> <path|->,
                          job-teed <job.json> <path|->,
                          jsonl <path|->, verify-bhtm-quote <att> <rd> <snp>
src/job.rs         runtime proof job validation/canonicalization
src/num3072.rs     3072-bit modular arithmetic
src/muhash.rs      Bitcoin Core-compatible MuHash accumulator
src/merkle_tree.rs CT-style append-only binding tree
src/stream.rs      stripped block delta verifier + slim builder
src/sev_snp_guest.rs   /dev/sev-guest ioctl wrapper (SNP_GET_REPORT 0xc020_5300)
src/verify_quote.rs    attestation/report-data/REPORT_DATA consistency checks
```

The compiled crate no longer depends on `bitcoin`, raw block parsing, RPC,
chain validation, or a UTXO set module.

## Attestation schema

`attestation.bin` byte layout (little-endian, version 1):

| Offset | Size | Field                  |
|-------:|-----:|------------------------|
| 0      | 2    | version (`u16 = 1`)   |
| 2      | 4    | start_height           |
| 6      | 4    | end_height             |
| 10     | 32   | start_block_hash       |
| 42     | 32   | end_block_hash         |
| 74     | 32   | reserved (always zero) |
| 106    | 32   | tree_root              |
| 138    | 8    | tree_size (`u64`)      |
| 146    | 384  | start_muhash           |
| 530    | 384  | end_muhash             |
| **914 total** | | |

The 32-byte slot at offset 74 is reserved (always zero in v1). It
preserves byte-layout stability for any tooling written against
earlier drafts.

For v1, the 64-byte `report-data.bin` is
`SHA512("BitcoinPIR/blockhash-to-muhash/report-data/v1" || attestation.bin)`,
identical for every chunk.

`attestation.bin` byte layout (little-endian, version 2, generic
job-bound UKI path):

| Offset | Size | Field                  |
|-------:|-----:|------------------------|
| 0      | 2    | version (`u16 = 2`)   |
| 2      | 2    | job_version            |
| 4      | 4    | start_height           |
| 8      | 4    | end_height             |
| 12     | 32   | start_block_hash       |
| 44     | 32   | end_block_hash         |
| 76     | 32   | job_sha256             |
| 108    | 32   | tree_root              |
| 140    | 8    | tree_size (`u64`)      |
| 148    | 384  | start_muhash           |
| 532    | 384  | end_muhash             |
| **916 total** | | |

For v2, the 64-byte `report-data.bin` is
`SHA512("BitcoinPIR/blockhash-to-muhash/report-data/v2" || attestation.bin)`.

The generic UKI measures only the proof runner and generic networking/job
delivery controls. It waits for a runtime `job.json` (or optionally fetches
one from a stable `bhtm.job_url`), canonicalizes it inside the guest, verifies
the compressed stream hash from the job, runs `job-teed`, uploads the proof
artifacts, and powers off. Use `scripts/build_bhtm_proof_uki.sh`
to build `/tmp/bhtm-proof-generic.efi`; do not put per-chunk stream URLs or
MuHash anchors into the UKI cmdline if the measurement must remain stable
across chunks.

Each chunk is proven **independently**. A client who knows a single
(height, block_hash, muhash) triple for any height in the chunk can:
1. Locate the chunk that covers that height (out of all archived chunks).
2. Verify the chunk's TEE quote pins `start_height` ≤ height ≤ `end_height`.
3. Compute the Merkle leaf `SHA256(height || block_hash || muhash)`.
4. Verify the leaf against `tree_root` (this can be done against the
   published `tree_root` from the manifest; the TEE quote only needs
   to bind `tree_root`, not every leaf).

No sibling-chunk attestation is required to trust any individual
chunk.

## VPSBG Non-TEE Smoke

Use this before wrapping the builder in TEE plumbing:

```bash
scripts/vpsbg-nontee-smoke.sh
```

The script stages the repo under `/root/blockhash-to-muhash-nontee`, builds and
tests it with the remote Rust toolchain, then runs the first-10k real-data
compact stream with `--expect-end-core-muhash`.

First successful VPSBG run:

- input sha256:
  `6331be35d836ef7769d44a16fa93c813d741798be79b3b49962271d0e039b06b`
- end height: `9999`
- end block hash:
  `00000000fbc97cc6c599ce9c24dd4a2243e2bfd518eda56e1d5e47d29e29c3a7`
- end Core MuHash:
  `fa8bc3bb3e3fb84e219da85b381cf140df1ee29d044272a6cac86e4a75c25cd4`
- attestation sha256:
  `2428bea5ee74e12c6eda0943cc8f6a85b0775ff5df6d59ffd899489ef6d548cb`
- TEE report data:
  `f7b8e78dba4774341a3fb1218af65eedee766f09204b5a0c2ebc1ae79103ee91dc056161aaae1da4769982081527edcb4b51ded4ed9ea590fab5c875fb59e499`
- wall time / RSS: `2:08.06`, `2396 KiB`

## VPSBG First-100k Streaming Run

The source-side exporter is `scripts/export-core-deltas.py`. For the first
100k run, it produced `/tmp/bhtm_first_100000.bin` and the stream was piped over
SSH into `target/release/blockhash-to-muhash bin -` on VPSBG. The compact stream
was not stored as a remote input file.

Core reference at height `99999`:

- block hash:
  `000000000002d01c1fccc21636b607dfd930d31d01c3a62104612a1719011250`
- Core MuHash:
  `f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4`
- txouts: `71885`

Measured source/export data:

- raw block input bytes: `59754905`
- compact stream bytes: `107356923`
- compact stream sha256:
  `612c3d62a3097151d4b7fed89a588bf541a797cc20dd96d13337ffe67a209384`
- export wall time: `14.57s`
- transactions / additions / removals: `216573` / `264247` / `192362`
- final source UTXOs: `71885`
- duplicate overwrites: `2`, `10000000000` sats

Measured VPSBG verifier result:

- stream wall time: `1368.72s`
- remote verifier wall time / RSS: `22:46.04`, `2948 KiB`
- tree root:
  `4ae1b5fcc5eebedefbbd9b8a28282c524601ee48ac74814b1b48e3f3bedec0ce`
- attestation sha256:
  `657b823f80dd031134534331197192bdd265900155b31c3262f62530f930285c`
- TEE report data:
  `49b9f9756cd2ce58011c0387cb6a911ab924feddbc3c6635085632b59ce732e5ea6c0d98cbbe1eb6ea049b33e54cacb8f4696660dd274d1d91c3803e64d45ada`

## Rolling 100k Archive

Use compact binary, not JSONL. Local staging is:
`/Volumes/Bitcoin/data/blockhash-to-muhash/stripped-deltas/`.
Hetzner archive storage is:
`pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/`.

Keep only one active chunk locally. After `rsync`, verify the remote compressed
stream with both `sha256sum` and `zstd -t`, then delete the local staging files.

Uploaded and verified on Hetzner:

- `mainnet-000000-099999/`
  - compressed bytes: `45016264`
  - compressed sha256:
    `4b63feb2d16664438a7b2d793e60de16a98922b08dc83e2dab491c60b7311b5a`
  - uncompressed bytes: `107356923`
  - uncompressed sha256:
    `612c3d62a3097151d4b7fed89a588bf541a797cc20dd96d13337ffe67a209384`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-100000-199999/`
  - compressed bytes: `2508868002`
  - compressed sha256:
    `ad200d135aaff0eb06707c52d51a5d50b6024689746ca57a0bfc88f86f436f57`
  - uncompressed bytes: `5540234865`
  - uncompressed sha256:
    `a9e944855f2286902fa45bfdd3c78a9b5aa9c5b5ed2a20a638f17be63e3403b0`
  - verified Core MuHash:
    `e1e783e9ac237240789cb97cf37b8e108c038b44b07e9c58768233bfb1a80f06`
  - verifier elapsed: `334.64s`
  - verifier attestation sha256:
    `3e4516758043ec2605ff615896912ff4b94a8051675ccb464ee9235c6de56e88`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-200000-299999/`
  - compressed bytes: `13244754182`
  - compressed sha256:
    `f10d30119ad5d3f00944271dd9e1ca64bd73a508128d873fa96e28c34586bbb5`
  - uncompressed bytes: `27856464980`
  - uncompressed sha256:
    `42f928f9eca9fc6f5eef8ff45a0a00624269c30fea7c37fa84c288467b57e5f5`
  - verified Core MuHash:
    `c8df37777612a7d513ffce0d17921cd0bc43e375f40f5f374c829dfbbd6cd3a0`
  - verifier elapsed: `1703.63s`
  - verifier attestation sha256:
    `8fe357b5098531cd49ebfe1f026f92886a0f35bc7e1c2489a478c26718fc9ea4`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-300000-399999/`
  - compressed bytes: `40195972526`
  - compressed sha256:
    `dee863e6f3f7274b3d46480d2da7f7161aa701efe24295a2f0fa62e581d4eed4`
  - uncompressed bytes: `76733738399`
  - uncompressed sha256:
    `1031a252864521407d9828533fa6bfeaa56f34b5fc12d3f0caa25f65da033360`
  - verified Core MuHash:
    `18cf1650a245e0979bfeb64495d04e39fa1dd1fbefca29aeb3bdecae7631ade1`
  - verifier elapsed: `1540.86s`
  - verifier tree root:
    `31516b07223100f319690f1b057227960549638cd879a311822f659c62fdeb2b`
  - verifier attestation sha256:
    `db018be0cfe67f1ed935a7c7e1f3cb73153235c941e6d81df0fd03d7c4e61298`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-400000-499999/`
  - compressed bytes: `93188176299`
  - compressed sha256:
    `0904a153f65cfdb2204357af04ceca469147db8ed8e67f97868e06015ffc0178`
  - uncompressed bytes: `152393244287`
  - uncompressed sha256:
    `c0d80facd804f5b43b439636bef38d984dbd940ee867d193c2da07d33245970b`
  - verified Core MuHash:
    `171e08e48bad8674528fb8d1a6fd9de42b4cfcdf78d8caf6558016fb9cb2a4ed`
  - verifier elapsed: `3027.89s`
  - verifier tree root:
    `38a0d50733e5209dcc9de2ab8e27551b3edf975e8dd96fdbc7eac52cf09a054c`
  - verifier attestation sha256:
    `355c7968f0cb8ecebc728f52ffcc5d9443ccdaef27387b0218563ee96fbfffe3`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-500000-599999/`
  - compressed bytes: `83295545724`
  - compressed sha256:
    `21bd7936a31c9c001156248927673a5c59a2a5fdb42d73528036065c9017c5fe`
  - uncompressed bytes: `145439276144`
  - uncompressed sha256:
    `d00ce37fcc9e791a64afb3687690dddf2ab10df6569716cb51dc8bf56d7dd1fb`
  - verified Core MuHash:
    `53882696420706cc51cedb6082413569196f667d9d4b8d87d6dc2a9abd6ac4ed`
  - verifier elapsed: `3117.18s`
  - verifier tree root:
    `f6abf2e16557a6ef9dd7373e955360ac1a1293f919cab65d2fee96224cabb08a`
  - verifier attestation sha256:
    `01d130858aa5f9edbc3683130e1204e2746eb27e1afed3a564755d94f84cd664`
  - manifest sha256:
    `58ddbd1e27650fc56b0096191f7a4454618cd283ce2ec0b85f05a7f03d15c7f3`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-600000-699999/`
  - compressed bytes: `96329808333`
  - compressed sha256:
    `d4cdc4f189cd8f7ad66073d15a0d4900e4644b999b15f8946af698c9ddc07849`
  - uncompressed bytes: `166494341612`
  - uncompressed sha256:
    `ba7282db8c2ecd40eea384d3b84c2256c10545285241fa64280168d464a11833`
  - verified Core MuHash:
    `5382e786ee8a1cc4e92d59030295e9cc0d3c0e3aea9c66e2fb9ee7cdae9d6f55`
  - verifier elapsed: `3832.89s`
  - verifier tree root:
    `a94b44aa8abbd882637ddc8fc9b798d0ae098e1422c2a9d50c10ce11aaf23a12`
  - verifier attestation sha256:
    `c76b04f4e0ba5b06f6d571dd1da71537d90f30019b9a3c18310bbb9655c8700d`
  - manifest sha256:
    `052492d0be6536bdf771fb83a1f0fa3dde751655e7a6e2b9ca3050009b9db443`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-700000-799999/`
  - compressed bytes: `82688792803`
  - compressed sha256:
    `706add584c24296b2548b63494a0858eb14c55c4b1a3e3e2718a7f5247152142`
  - uncompressed bytes: `159115007032`
  - uncompressed sha256:
    `6526a6bd489230e6424773ae81cddf02f57f5b45b9d67c01cb18198ea93dc0ad`
  - verified Core MuHash:
    `9226d85add58fe2635c468013944cdefb8fb8fae995888ea76621ff05ffc1c21`
  - verifier elapsed: `3961.08s`
  - verifier tree root:
    `ac3762584df067f34e46fbe5ad8ddc6ec3ead3d7073bc2d5f73fc06fbdd3411b`
  - verifier attestation sha256:
    `a931b22c893f7eec346aad8407c1e7cc12198bcadec935cb3ebee5c8b1107546`
  - manifest sha256:
    `f2d76737534aa997a121127602806bde46f40e8b7a5901af9c4ed38edac8ccee`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-800000-899999/`
  - compressed bytes: `81289881114`
  - compressed sha256:
    `efa4c3e14540e9777cc214159bceab74e82ba4ec13def3059e85bf3879d3a758`
  - uncompressed bytes: `198425503154`
  - uncompressed sha256:
    `db16be1463954c2ba260c222006094640450682c1fc2fb835b0452fca1521e14`
  - verified Core MuHash:
    `1ec8dedf1473bd17f421c1dd7c78d6ba61747ac3161c14a27322b528bb032ca9`
  - verifier elapsed: `5097.41s`
  - verifier tree root:
    `e3d61a7e18dc74ea798470917e40d8e578de946fe32e13022a8d986236dcc14b`
  - verifier attestation sha256:
    `4a95f3622ba857b7b5fea79fdd21e24740609d0ff40711d423a885d1e53d2b37`
  - manifest sha256:
    `410aad5211e7e0e04f633383c64f6aa2d5aa8e40186043befdb7dedf09d42297`
  - manifest includes the full 384-byte verifier `end_muhash`.
- `mainnet-900000-954920/`
  - current-head partial chunk, selected as a conservative tip-minus-6 anchor;
    this is not a canonical 100k boundary chunk.
  - compressed bytes: `40599117380`
  - compressed sha256:
    `432510fbcf72d53cf9bd08b2161c75aadc4edecf46cdfccc557858a9ff73776a`
  - uncompressed bytes: `99504614791`
  - uncompressed sha256:
    `0dc2718e186803fc77b4b3536d79802b8ff3407d04342f98e8a4580da5bd3a03`
  - verified Core MuHash:
    `c27e1fafa9dd154cfd16a78429477181febbecf49b3d49016c462b0764429470`
  - verifier elapsed: `2599.17s`
  - verifier tree root:
    `babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6`
  - verifier attestation sha256:
    `667605a6fe956cc3379d08a31dfefb7b7bdd03846d4440fd0e69466389c75759`
  - manifest sha256:
    `ab90de7b0744c86226fa14d3b3905b21e520933d9f5e0c45668e2e0243f5d45e`
  - export elapsed: `31803.82s`
  - manifest includes the full 384-byte verifier `end_muhash`.

Chunk 0 must use `utxo-scan` from genesis because it covers the historical
BIP30 duplicate-txid overwrite heights. Later chunks should use
`--source-mode prevout-rpc`, which reads Bitcoin Core verbosity-3 prevout data
and avoids storing a source-side UTXO map. `prevout-rpc` defaults to smaller
RPC batches to avoid large JSON memory spikes.

`mainnet-900000-954920` is a verified current-head partial archive. Use
`mainnet-900000-954920/manifest.json` and its full 384-byte `end_muhash` as the
trusted anchor for any continuation from height `954921`. If a canonical
`mainnet-900000-999999` archive is needed later, regenerate or explicitly treat
this partial chunk as a different artifact.

## MuHash

- Prime: `p = 2^3072 - 1103717`
- Coin serialization follows Bitcoin Core coinstats:
  `COutPoint || uint32(height << 1 | coinbase) || CTxOut`
- Hash-to-element follows Bitcoin Core:
  `SHA256(serialized_coin)` as ChaCha20 key, zero nonce,
  384-byte little-endian `Num3072`
- Run endpoints expose the full normalized 384-byte MuHash value.
- `MuHash::commitment()` returns the Core 32-byte SHA256 commitment when needed.
- Removals stay in the denominator. The streaming path snapshots un-inverted
  `(numerator, denominator)` block states and batch-inverts pending
  denominators before appending MuHash tree leaves; do not reintroduce
  per-block `to_bytes()` normalization in the main loop.

## Storage

The builder has low storage requirements. It keeps only:

- MuHash accumulator
- MuHash Merkle tree frontier
- counters / last-block metadata

It does not store blocks and does not store the UTXO set. Large block storage and
Core undo/chainstate access belong on the data source machine.

## Test Commands

```bash
cargo test
cargo test --test integration
cargo test --test sev_snp_smoke
```

## Environment

- Workspace: `/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash`
- Data source used during earlier experiments: `/Volumes/Bitcoin/bitcoin`
