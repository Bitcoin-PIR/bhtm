# Hetzner SEV-SNP Proof Experiment — Runbook

**Goal.** Run the smallest archived chunk (`mainnet-000000-099999`)
through the reusable generic SEV-SNP proof-runner UKI on the real
SEV-SNP VPSBG host (`vpsbg-pir`, IP 87.120.8.198), submitting
chunk-specific data as runtime `job.json`. The output proof must bind
`job_sha256` in attestation v2 and include a real 1184-byte
`sev-snp-report.bin` whose `MEASUREMENT` is stable across different
chunk jobs.

**Why this chunk.** Smallest by data size (~45 MB compressed, ~107 MB
uncompressed), runs in ~5 minutes. Genesis-anchored (anchor =
`before-genesis`), so it's the simplest case.

## Pre-known facts (do not re-derive)

- Chunk: `mainnet-000000-099999`
- Hetzner archived at:
  `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/`
- Stream URL (operator must choose — see below):
  needs to be **HTTPS** to a host the VPSBG guest can reach
- Stream SHA256: `4b63feb2d16664438a7b2d793e60de16a98922b08dc83e2dab491c60b7311b5a`
  (compressed)
- Expected end Core MuHash (display hex):
  `f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4`
- Verifier elapsed (non-TEE baseline): n/a for chunk 0
- blockhash-to-muhash binary SHA (Linux/x86_64, Rust 1.94.1, generic
  job/v2 raw-report proof source):
  `6811c23193e9392367856ca6d2149b6f772857d199340d36282d8df1d70170e6`
  - Older values `976d5c51...`, `86517815...`, `468de531...`, and
    `b3185fc...` are stale for this proof path. Do not use the old tiny UKI.
- Built under toolchain `1.94.1` (pinned in
  `/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash/rust-toolchain.toml`)
- VPSBG kernel: `7.0.0-15-generic` (Ubuntu 25.04 plucky-backport)
- Generic UKI build (canonical): `scripts/build_bhtm_proof_uki.sh`
- Step-by-step portal runbook:
  `scripts/bhtm_proof_one_chunk.sh`
- HTTPS backend endpoint:
  `scripts/bhtm-proof-endpoint.py`, deployed as
  `pir-hetzner:/root/blockhash-to-muhash/proof-endpoint/bhtm_endpoint.py`.
  This must be the v2 multi-chunk endpoint; the older endpoint only served
  chunk 0 and only accepted `/proof/` uploads, which is insufficient for
  generic `job.json` proofs.
- Generic UKI artifact:
  - local: `/tmp/bhtm-proof-generic.efi`
  - local streaming copy: `/tmp/bhtm-proof-generic-streaming.efi`
  - remote: `pir-hetzner:/tmp/bhtm-proof-generic.efi`
  - UKI SHA256:
    `88054be8b91143209a753a7d5ac887e966a3829ea85d8881f5fc6b37f1b90700`
  - cmdline:
    `rdinit=/sbin/bhtm-proof-init console=ttyS0,115200 console=tty1 quiet loglevel=3 bhtm.job_port=8080 bhtm.static_ip=87.120.8.198/32 bhtm.gateway=172.16.0.1 bhtm.dns=9.9.9.9`
  - note: this is the streaming rebuild for chunk 2+. The previous
    `a40fd434...` UKI proved chunks 0 and 1 but required tmpfs space for both
    compressed and decompressed inputs.
- Chunk-0 runtime job:
  - raw: `/tmp/bhtm-mainnet-000000-099999.job.json`
  - canonical: `/tmp/bhtm-mainnet-000000-099999.canonical.job.json`
  - job SHA256:
    `c600e1e97228797494cc6b55d2559e640220d829d9459db787b8a191ce35782d`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-000000-099999/proof-generic`
- Chunk-1 runtime job for stable-measurement comparison:
  - raw: `/tmp/bhtm-mainnet-100000-199999.job.json`
  - canonical: `/tmp/bhtm-mainnet-100000-199999.canonical.job.json`
  - job SHA256:
    `2634e4e2599ecef17ce3887adab2e914d79c6a3e242ecc9261d60aaf7631d16d`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-100000-199999/proof-generic`
- Chunk-0 streaming-UKI redo:
  - remote proof:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/proof-generic/`
  - old fixed-image proof archive:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/proof-generic-fixed-report-uki-a40fd434-20260625/`
  - updated manifest SHA256:
    `991df594eecbb517803f1d48ad978453597d8aea92f1b7f49abd110551326ea4`
  - proof backup SHA256:
    `e6eff272c2afe509ad1cfe2f3e495c1125c8d6f85a39c7ee2eca210b3768e8a2`
  - SEV-SNP report SHA256:
    `3a4e0b87b2f8b3d002c0efa3f384f42106310242141b9ec9388e1d9bcb0d572f`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
- Chunk-1 streaming-UKI redo:
  - old fixed-image proof archive:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-100000-199999/proof-generic-fixed-report-uki-a40fd434-20260625/`
  - remote proof:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-100000-199999/proof-generic/`
  - updated manifest SHA256:
    `57dc1f88464adbc39b6c4edcc48a9fa899a3e3193dfa3d814edd92e38ab11023`
  - proof backup SHA256:
    `f7b9dcaf024d1cb2c3475d056e125f0d7f2dd70242627400cbc03eae017ab1e5`
  - SEV-SNP report SHA256:
    `d1d9686084eee3a978897ba630f6427160346db20cd59fce72c1dd838a259d0e`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
- Chunk-2 runtime job and streaming-UKI proof:
  - raw: `/tmp/bhtm-mainnet-200000-299999.job.json`
  - canonical: `/tmp/bhtm-mainnet-200000-299999.canonical.job.json`
  - job SHA256:
    `928dacf132aa69bcd3706423e2660457739362ac1432274f79b9d21e89e9118a`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-200000-299999/proof-generic`
  - proof remote:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-200000-299999/proof-generic/`
  - updated manifest SHA256:
    `9de1786ee47e879f3577d651d68938d2eafb52b59ccdaef76e96f5d6d6b4fcc1`
  - proof backup:
    `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-200000-299999.proof-generic-and-manifest.tar.zst`
  - proof backup SHA256:
    `5acaf791ae53933678f6d9d4ea3a0de8e610d391169c9c08108a31fbf1b1e480`
  - attestation SHA256:
    `e07aef592d575baad72367697cde61ba0578d0b17a49674beef2b802748c2896`
  - SEV-SNP report SHA256:
    `f1218cc8395ccb48ecbe48ace9cde967db25e7e00d46401a99b80925a7b35c74`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - REPORT_DATA:
    `f6022c57b13b6d6a717c60911d921933bdfd259d2790b0e8789f445f96dea99c16570d27d2664b910a9ebf75c4871b3a2a35551dbbbff7809b5e72e262359a46`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
- Chunk-3 runtime job and streaming-UKI proof:
  - raw: `/tmp/bhtm-mainnet-300000-399999.job.json`
  - canonical: `/tmp/bhtm-mainnet-300000-399999.canonical.job.json`
  - job SHA256:
    `7b2e3c542f08b633e6f9b2a3959277ad4841230ca660afd17ea630ae02cafb05`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-300000-399999/proof-generic`
  - remote prepared job:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-300000-399999/proof-generic/job.prepared.json`
  - proof remote:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-300000-399999/proof-generic/`
  - updated manifest SHA256:
    `494b71c969ac85d1d99304a01622dca4b85d22a930a4efd4ecc2ad605a7ac927`
  - proof backup:
    `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-300000-399999.proof-generic-and-manifest.tar.zst`
  - proof backup SHA256:
    `24a8da3d25d2b58633d44d5bcbf04ad8d00d14843966a443c0a1784010b32c25`
  - attestation SHA256:
    `242569a2414e535b89cfc71a82535a7ce3da2ba77e69dba788709e2931ecd5f9`
  - SEV-SNP report SHA256:
    `ae1dcdaddfc8197389f0d441511163552ee36b0d67056709c68e4fac69437fcd`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - REPORT_DATA:
    `9ad1541fe0e17955acfbc72ee74583d30a5870e70fe73af49db09435a993f97473ae0fdc5b3927c81450e6f9985d80527f6082a28b7dda267efbfbd16c57faf8`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
- Chunk-4 runtime job and streaming-UKI proof:
  - raw: `/tmp/bhtm-mainnet-400000-499999.job.json`
  - canonical: `/tmp/bhtm-mainnet-400000-499999.canonical.job.json`
  - job SHA256:
    `2cdaccd7ec96c87f828b60eaa0ccbe606be76165114a6289b74fc82de8f16f7f`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-400000-499999/proof-generic`
  - remote prepared job:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-400000-499999/proof-generic/job.prepared.json`
  - proof remote:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-400000-499999/proof-generic/`
  - updated manifest SHA256:
    `26cf5f45f9baae485fcebd450c7a2446080a1e5a93f252ac3347d389452fbc10`
  - proof backup:
    `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-400000-499999.proof-generic-and-manifest.tar.zst`
  - proof backup SHA256:
    `977e645e36eb926acd71fb8238ca869a8188f33f2d577dbf9b7fe232e142540f`
  - attestation SHA256:
    `57b9f69a37021c785748b73bad11fd761ee2bdd8e64a7058575c8cc73caeea58`
  - SEV-SNP report SHA256:
    `be4b189af00a0eb9c3b240b85c8d78ed9d0e211c3ad0ea8ba462df8227bdf40f`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - REPORT_DATA:
    `f6969f606c149536ce60b3c87524e3755b6a39f26cd5714b933624ed316bf5243ac21ca9df13e7a7ddb02e4fb1f686ffc9510281d41ef0cb04b4b9949683b5d0`
  - end Core MuHash:
    `171e08e48bad8674528fb8d1a6fd9de42b4cfcdf78d8caf6558016fb9cb2a4ed`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
- Chunk-5 runtime job and streaming-UKI proof:
  - raw: `/tmp/bhtm-mainnet-500000-599999.job.json`
  - canonical: `/tmp/bhtm-mainnet-500000-599999.canonical.job.json`
  - job SHA256:
    `37665da267352a4b5569365a1410feb2871959ccd9a952f7f9dccce82408afb1`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-500000-599999/proof-generic`
  - remote prepared job:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-500000-599999/proof-generic/job.prepared.json`
  - proof remote:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-500000-599999/proof-generic/`
  - updated manifest SHA256:
    `1ef3d7659a9810317d81ce5d95e98f177c5b345feca7c1ca5174ce5a356c2014`
  - proof backup:
    `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-500000-599999.proof-generic-and-manifest.tar.zst`
  - proof backup SHA256:
    `47cc73801cdc8e85cb056c2ae8c87ca96983b6d25376874cc620f83185e069f7`
  - attestation SHA256:
    `634438d1b9e9eec5af63ae440b21463cb2aee3ca20b54750910f08eb0677a544`
  - SEV-SNP report SHA256:
    `52955a379d9657794a7a4e31fa4621ccd6ccc83edc220241801ab79b8a49af1c`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - REPORT_DATA:
    `05f94c427ae8c84513dadf70ec0c7d6961bfe442ea519c670fb4268470cfcd510d7e7ebf603cfd232fc22a43c4961736e984e30f99eab3e24867aa471da36e82`
  - end Core MuHash:
    `53882696420706cc51cedb6082413569196f667d9d4b8d87d6dc2a9abd6ac4ed`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
- Chunk-6 runtime job and streaming-UKI proof:
  - raw: `/tmp/bhtm-mainnet-600000-699999.job.json`
  - canonical: `/tmp/bhtm-mainnet-600000-699999.canonical.job.json`
  - job SHA256:
    `923ce6f2949b73d5397c9964b15260a60eb1047da2187592ed7ca2ae31c838a6`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-600000-699999/proof-generic`
  - remote prepared job:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-600000-699999/proof-generic/job.prepared.json`
  - proof remote:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-600000-699999/proof-generic/`
  - updated manifest SHA256:
    `bd83c3605af1868eae285d023b65f95ba4956c165a196611864f004731448396`
  - proof backup:
    `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-600000-699999.proof-generic-and-manifest.tar.zst`
  - proof backup SHA256:
    `6c35f4ec5166f93122e5d1e391e24d781abf37edd7351aebc0d14908f58f16a9`
  - attestation SHA256:
    `3a6a0dc291c9410ac9f9892adcab8f793b7e3b5134ed6cbecead527fcf256abe`
  - SEV-SNP report SHA256:
    `57161965257eef033abcf09a30898d8a4c76a0e6dab434d197cfe729a0894bfa`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - REPORT_DATA:
    `bafa78086189910f66cd979ae1b8ab0680c185707520fb083c6355ae38d04e2f2eb927989709b65de11ea01dd428b47f7c0003854ee15a74074cef2c464b5b00`
  - end Core MuHash:
    `5382e786ee8a1cc4e92d59030295e9cc0d3c0e3aea9c66e2fb9ee7cdae9d6f55`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
- Chunk-7 runtime job and streaming-UKI proof:
  - raw: `/tmp/bhtm-mainnet-700000-799999.job.json`
  - canonical: `/tmp/bhtm-mainnet-700000-799999.canonical.job.json`
  - job SHA256:
    `faeb2560210b800cf4d1fb1c949e2d3b7f29e33cd70666f0700f0d5c4b47269b`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-700000-799999/proof-generic`
  - remote prepared job:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-700000-799999/proof-generic/job.prepared.json`
  - proof remote:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-700000-799999/proof-generic/`
  - updated manifest SHA256:
    `a954787a52d1612b300767d05ce7b3208e03609b103b0b3d185dadbbd49ba604`
  - proof backup:
    `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-700000-799999.proof-generic-and-manifest.tar.zst`
  - proof backup SHA256:
    `6557cdd3ad73423d6b2e810493c00bb7ff64f5439c5c53d4f4b8c420dec93c2b`
  - attestation SHA256:
    `00ce6b5527b2399fd8ef5c92c2f982859768919daa9e516ca81f2bd542dad7ec`
  - SEV-SNP report SHA256:
    `00caa62f032dd8f3222366f163c4e56f17e75474bb5e917e9fb40a7076319afa`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - REPORT_DATA:
    `09ffb64c1947fdb8782f60823027c708b004d19ebb3963bc847a48b5e196ee0c1b4b01b142feab33e71655ddeba8f5e4e7a0097388fed0a139f699cdeed0fd21`
  - end Core MuHash:
    `9226d85add58fe2635c468013944cdefb8fb8fae995888ea76621ff05ffc1c21`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
- Chunk-8 runtime job and streaming-UKI proof:
  - raw: `/tmp/bhtm-mainnet-800000-899999.job.json`
  - canonical: `/tmp/bhtm-mainnet-800000-899999.canonical.job.json`
  - job SHA256:
    `804ac08a819709691475ca6f6c12bb499f4349f3bb189eb3effadec2ec7d8ae1`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-800000-899999/proof-generic`
  - remote prepared job:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-800000-899999/proof-generic/job.prepared.json`
  - proof remote:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-800000-899999/proof-generic/`
  - updated manifest SHA256:
    `e1fb14ff7072418555984c1cef2ea387bbf91ae372ca5269211ed377181dd69e`
  - proof backup:
    `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-800000-899999.proof-generic-and-manifest.tar.zst`
  - proof backup SHA256:
    `23375569a16eacaf338f6c04bf6f6f37ff5c03448634757ad7b450cb2b205687`
  - attestation SHA256:
    `b7b13d9061ed3b9a0766888a9ecbae977bb3ba1a849143b29500f873fe08d7f1`
  - SEV-SNP report SHA256:
    `e1dadc817aff72ba1156c4ed3d8428bf5b46603326c1fb9e83c4a1e98f287a61`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - REPORT_DATA:
    `30090fa7cad97891327221bb8d60bd71e31ae9005564f3376bd0975728e8670994701e2e4138315878aa4c5b9792751e51ca488c6f2537587c7ecfc332d31b7b`
  - end Core MuHash:
    `1ec8dedf1473bd17f421c1dd7c78d6ba61747ac3161c14a27322b528bb032ca9`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
  - submitted via direct guest PUT:
    `curl -X PUT --data-binary @/tmp/bhtm-mainnet-800000-899999.canonical.job.json http://87.120.8.198:8080/job`
  - accepted response: `ok`
  - first Hetzner stream check:
    `2026-06-25T16:04:30Z stream bytes_sent=1022947864 pct=1.258`
- Partial current-head chunk `mainnet-900000-954920` runtime job and
  streaming-UKI proof:
  - raw: `/tmp/bhtm-mainnet-900000-954920.job.json`
  - canonical: `/tmp/bhtm-mainnet-900000-954920.canonical.job.json`
  - job SHA256:
    `19e03b71ea9150b1c64d4a0069469384420a22dd4bcffd0ae1300a7121331e52`
  - upload URL:
    `https://65-21-91-217.sslip.io/mainnet-900000-954920/proof-generic`
  - remote prepared job:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-900000-954920/proof-generic/job.prepared.json`
  - proof remote:
    `pir-hetzner:/root/blockhash-to-muhash/stripped-deltas/mainnet-900000-954920/proof-generic/`
  - updated manifest SHA256:
    `11286a0ee29edb10c5f9b8383dece85fad8183136f41035796eb2a8fa1b95f6f`
  - proof backup:
    `pir-hetzner:/root/blockhash-to-muhash/proof-backups/generic-uki-streaming-20260625/mainnet-900000-954920.proof-generic-and-manifest.tar.zst`
  - proof backup SHA256:
    `f9f30ff8248a800ff7ade3985681d25f6356a7d24359ed68e14caa1a1941580b`
  - attestation SHA256:
    `4c360e56c50f0d38747682932e35d9f54ce9992893bf7b61bbe62d16f37158ed`
  - SEV-SNP report SHA256:
    `383523d838d36d8aefedf57fc938b209c6d1b9cd24d9f23c5cb68b0ea7da03c6`
  - MEASUREMENT:
    `652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279`
  - REPORT_DATA:
    `6c8f972108a26b2d17ec1e5741c9134449db7fb1dca6a97d8d9b8a51ec1557d75fc905bdf67ecdf9e102908498eaa033a82a78f1910806d66d619412074836b1`
  - expected end Core MuHash:
    `c27e1fafa9dd154cfd16a78429477181febbecf49b3d49016c462b0764429470`
  - verification: `verify-bhtm-quote` passed and AMD VCEK/report signature
    verification passed.
  - chunk 0 and chunk 1 were later redone with the same streaming UKI, so the
    full proof set uses one image measurement.

## What needs to happen before the experiment

1. **Pick a stream URL the VPSBG guest can reach.** The chunk file is
   on `pir-hetzner` but the SEV-SNP guest (`vpsbg-pir`) needs to
   fetch it over HTTPS. Current direct Hetzner endpoint:
   `https://65-21-91-217.sslip.io/mainnet-000000-099999/stream.bhtmbin.zst`.

2. **Pick an attest URL.** The init script will `curl --upload-file`
   the output files (job.json, attestation.bin, report-data.bin,
   sev-snp-report.bin, stats.json, run.stdout) to
   `<attest_url>/<filename>`.
   Current direct Hetzner endpoint:
   `https://65-21-91-217.sslip.io/mainnet-000000-099999/proof-generic`.

3. **Confirm `blockhash-to-muhash` is built on the Linux/x86_64 build host.**
  ```bash
  ssh pir-hetzner 'cd /root/blockhash-to-muhash/verifier-src && \
    cargo build --release && \
    sha256sum target/release/blockhash-to-muhash'
  ```
  Output must be `6811c23193e9392367856ca6d2149b6f772857d199340d36282d8df1d70170e6`.

## Run the experiment

Build/select the generic UKI, reboot, then submit the runtime job. The helper
prints the exact commands and writes a job file:

```bash
ssh root@vpsbg-pir '/root/bhtm/scripts/bhtm_proof_one_chunk.sh \
  mainnet-000000-099999 \
  https://65-21-91-217.sslip.io/mainnet-000000-099999/stream.bhtmbin.zst \
  4b63feb2d16664438a7b2d793e60de16a98922b08dc83e2dab491c60b7311b5a \
  https://65-21-91-217.sslip.io/mainnet-000000-099999/proof \
  f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4'
```

The script will:
- Write `/tmp/bhtm-mainnet-000000-099999.job.json`.
- Show the generic UKI build invocation.
- Show the VPSBG portal upload/select/reboot steps.
- Show the `curl --request PUT --data-binary @job.json
  http://87.120.8.198:8080/job` submission.
- Show the verification command.

Or use the local helper once the UKI is booted:

```bash
scripts/submit-generic-proof-job.sh \
  /tmp/bhtm-mainnet-000000-099999.canonical.job.json \
  /tmp/bhtm-proof-mainnet-000000-099999-generic
```

Total wall time: ~10–15 min (mostly portal ops; chunk 0 proof computation is
small once the compressed stream is local to Hetzner/VPSBG networking).

## Recovery if anything goes wrong

```bash
ssh root@vpsbg-pir '/root/bhtm/scripts/bhtm_proof_one_chunk.sh ...'
# If the run stalls or errors, the script aborts.
# Recovery:
#   VPSBG portal → Measured Boot → UKI → set to "None" → Save & Reboot
#   Stock Ubuntu rootfs boots with sshd.
```

### Previous per-chunk UKI artifact

This artifact was useful for the first proof but is superseded by the generic
UKI path. Do not use it as the reusable production image.

- Local upload artifact:
  - `/tmp/bhtm-proof-mainnet-000000-099999.efi`
  - bytes: `114,080,256`
  - SHA256: `8be1781237c06dc71254fe6ab00b29216df2dd9457c1be038112fbf2d7d22dbe`
- Remote build artifact:
  - `vpsbg-pir:/tmp/bhtm-proof-mainnet-000000-099999.efi`
- This artifact was built with:
  - `KERNEL=/boot/vmlinuz-7.0.0-15-generic`
  - `BLOCKHASH_TO_MUHASH_BIN=/root/blockhash-to-muhash-proof/target/release/blockhash-to-muhash`
  - `BHTM_CHUNK_URL=https://65-21-91-217.sslip.io/mainnet-000000-099999/stream.bhtmbin.zst`
  - `BHTM_STREAM_SHA256=4b63feb2d16664438a7b2d793e60de16a98922b08dc83e2dab491c60b7311b5a`
  - `BHTM_ATTEST_URL=https://65-21-91-217.sslip.io/mainnet-000000-099999/proof`
  - `BHTM_EXPECT_END_CORE_MUHASH=f7aaa47daff9632e6327bc6cc89bb276cbfd6dc16b2f9811a1581675239613f4`
  - `BHTM_STATIC_IP=87.120.8.198/32`
  - `BHTM_GATEWAY=172.16.0.1`
  - `BHTM_DNS=9.9.9.9`
- Direct HTTPS is served by `bhtm-caddy.service` on `pir-hetzner`,
  reverse-proxying the local proof endpoint on `127.0.0.1:18080`.
  The old quick `trycloudflare` tunnel is intentionally not used.

## What "success" looks like

After Step 6, you should have:

1. `/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/proof-generic/`
   containing:
   - `job.json`
   - `attestation.bin` (916 B)
   - `report-data.bin` (64 B)
   - `sev-snp-report.bin` (1184 B)
   - `stats.json`

2. `blockhash-to-muhash verify-bhtm-quote` prints:
   ```
   status=ok
   attestation_version=2
   job_version=1
   job_sha256=<matches SHA256(canonical job.json)>
   start_height=0
   end_height=99999
   start_block_hash_internal=...
   end_block_hash_internal=...
   tree_root=...
   tree_size=100000
   expected_report_data=...
   report_data_field_in_sev_snp_report=...
   report_data_match=ok
  WARNING: AMD ECDSA signature inside the SEV-SNP report has NOT been verified.
  ```

Current fixed/raw-report runs:

```text
mainnet-000000-099999:
job_sha256=c600e1e97228797494cc6b55d2559e640220d829d9459db787b8a191ce35782d
attestation_sha256=a6b45fb38c98a79a175bae879069a31301c9316cd2c605c1930e5278d7f5daab
sev_snp_report_sha256=c2727f95c3f36d069ecea435cd91efb26d6f62f0661764924847f0a48c338eef
measurement=9373e779635cecb22bfde72250e4e27c887758d4e7bbc0bdf31b4b2c33bf46964a1587b921c2002faff256dfd980fd7d
report_data=364d14da44b114f3928d513cce16a3f4e672aa66647d1c951fd371b368d678ea43067d413e1146a002d4dfcaf3526ce1ceb7329af808c4e3097b893786ab9372
amd_chain_and_report_signature=ok
manifest_sha256=743593580e3ad61a324da00b0acc805c752858a75bec2240720bc53325e28948

mainnet-100000-199999:
job_sha256=2634e4e2599ecef17ce3887adab2e914d79c6a3e242ecc9261d60aaf7631d16d
attestation_sha256=256d2e168d58e8844c8a6d8f7ed5757a0836f01e6489c6c41a02d684b4ca848f
sev_snp_report_sha256=1bf270eadfd8fb576cf563dcd81b58baaeb0e9d9a620ae8da4975ba976b7c893
measurement=9373e779635cecb22bfde72250e4e27c887758d4e7bbc0bdf31b4b2c33bf46964a1587b921c2002faff256dfd980fd7d
report_data=f99274892cdb334ee4930dbcdd97fe932a12c647d4bb1b6520e0d19d692a05f03b7a23ee6e2e07b70649a53ee4c1265a9c649861670a8ac960b5b94fbd495b65
amd_chain_and_report_signature=ok
manifest_sha256=b6e5b7b3832e530c92f3a9951a53bdefdcee8451253fc4e2723e1bc061a490a0
```

The AMD KDS VCEK URL must use the short non-zero chip ID
`003642735DDC6E02`; the zero-padded 64-byte chip ID returns 404.

3. The SEV-SNP report's `MEASUREMENT` field (offset 0x90, 48 bytes) is
   a non-zero SHA-384 — paste it into the chunk manifest as the
   authoritative `tee_image_measurement`.

## What the report lets us verify

- `REPORT_DATA` (offset 0x50, 64 B) equals
  `SHA512("BitcoinPIR/blockhash-to-muhash/report-data/v2" || attestation.bin)`
  → the guest actually computed the attestation we asked for.
- `MEASUREMENT` (offset 0x90, 48 B) is the SHA-384 of the measured
  initial guest state → the image running inside the TEE matches the
  published binary SHA chain.
- `HOST_DATA` (offset 0xc0, 32 B) is informational, not security-bearing.
- The ECDSA signature (offset 0x2a0, 512 B) chains to AMD's VCEK for
  this chip + TCB → third-party verification via `pir-attest-verify`
  (not part of this experiment).

## Report template after a successful run

```text
=== Hetzner SEV-SNP proof run: mainnet-000000-099999 ===
Date: YYYY-MM-DD
Operator: <name>
VPSBG instance: vpsbg-pir (87.120.8.198)

blockhash-to-muhash binary SHA:
  6811c23193e9392367856ca6d2149b6f772857d199340d36282d8df1d70170e6

UKI SHA256 (built UKI sha256sum):
  <computed at build time>

MEASUREMENT (48 bytes hex, from report offset 0x90):
  <computed at run time>

REPORT_DATA (64 bytes hex, from report offset 0x50):
  <computed at run time>

verify-bhtm-quote output:
  status=ok
  attestation_version=2
  job_sha256=<matches job.json>
  tree_root=<matches existing manifest.json tree_root>
  end_muhash=<matches existing manifest.json end_muhash>
  report_data_match=ok

Wall time: ~N minutes
```

## After the experiment

Update `/root/blockhash-to-muhash/stripped-deltas/mainnet-000000-099999/manifest.json`
with the new fields:

```json
{
  "sev_snp_report": "proof-generic/sev-snp-report.bin",
  "sev_snp_report_bytes": 1184,
  "sev_snp_report_sha256": "<computed>",
  "attestation_sha256": "<computed>",
  "attestation_version": 2,
  "job": "proof-generic/job.json",
  "job_sha256": "<computed>",
  "tee_image_measurement": "<48-byte hex>",
  "tee_report_data": "<64-byte hex>"
}
```

If the experiment succeeds, repeat by submitting different `job.json` files to
the same generic UKI. The SEV-SNP `MEASUREMENT` should remain identical across
chunks; only `job_sha256`, chunk outputs, tree root, and end MuHash should
differ.
