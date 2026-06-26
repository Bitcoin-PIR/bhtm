# Prompt: Add BitcoinPIR Database Authenticity Trust Chain To Website And Frontend

You are working on BitcoinPIR. Please update the public educational website and
the production browser frontend so users can load, verify, and understand the
new database-authenticity trust chain:

```text
PIR answer / database root
  -> attested database build proof
  -> snapshot anchor: height, block_hash, MuHash
  -> blockhash-to-MuHash leaf proof
  -> BHTM chunk tree root
  -> AMD SEV-SNP quote for the reusable BHTM proof-runner image
```

This is not the same as the existing "query privacy" verification story. Keep
the privacy proof and the database-authenticity proof conceptually separate in
the UI and docs.

## Repos And Paths

Use the correct trees:

- Educational website:
  `/Users/cusgadmin/bitcoin-pir/website`
- Production browser frontend/runtime repo:
  `/Users/cusgadmin/BitcoinPIR`
- Blockhash-to-MuHash proof repo and handoff docs:
  `/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash`

Read these first:

- `/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash/docs/WEBSITE_DATABASE_AUTHENTICITY_SUMMARY.md`
- `/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash/TRUST-CHAIN-948454.md`
- `/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash/AGENTS.md`
- `/Users/cusgadmin/BitcoinPIR/web/src/db-proof.ts`
- `/Users/cusgadmin/BitcoinPIR/web/src/attest-pin.ts`
- `/Users/cusgadmin/BitcoinPIR/web/reproduce.html`

The current website structure is component-driven:

- `/Users/cusgadmin/bitcoin-pir/website/src/data/sections.ts`
- `/Users/cusgadmin/bitcoin-pir/website/src/components/sections/`

The current production frontend already has DB proof status plumbing:

- `web/src/db-proof.ts`
- `web/src/attest-pin.ts::PRODUCTION_DB_PROOF_PINS`
- `web/index.html` DB proof badges
- `web/reproduce.html` reproducibility section

## Current Concrete Witness

The first end-to-end trust-chain witness is for the production DB proof anchor:

```text
height:
  948454
block_hash:
  00000000000000000001ef683c02c383315db7e917c69d20f79e05985560a4e4
Core-display MuHash:
  cf4fc1f1dd400622a5b6f39eca7f764a30570c30cc668e04f00e8a3356c2a2ee
bucket_super_root:
  e2ba2eee6788424309a95f771893d5401cc8e3ceec6188dc2708900e211a910a
onion_super_root:
  f86baa3966a61cdcd70d8c0ad9bed233f591806eb351db2ae35ac0192a3fe997
BHTM chunk:
  mainnet-900000-954920
BHTM tree_root:
  babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6
BHTM leaf_hash:
  0171a475dc43b97db594c7512039d12f5213d270b42e56babfa44d65359eeaaf
BHTM leaf_index:
  48454
BHTM tree_size:
  54921
BHTM streaming UKI measurement:
  652f8c813382abb0c09a7bdc6528e6a3494f9f0833eb087349ad5dac76dec140532763fec8eeaf3fa52723b6a5de8279
```

The local trust-chain check is recorded at:

```text
/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash/TRUST-CHAIN-948454.md
```

It verified:

- the BHTM leaf proof for height `948454`;
- the BHTM leaf proof against the BHTM chunk tree root;
- the same BHTM tree root against the SEV-SNP proof for
  `mainnet-900000-954920`;
- the AMD VCEK/report signature for the BHTM proof;
- the existing DB proof's acceptance of the same block hash and MuHash.

## Current Proof Artifact Locations

Operator/archive paths on `pir-hetzner`:

```text
/root/blockhash-to-muhash/stripped-deltas/mainnet-900000-954920/proof-generic/
  attestation.bin
  report-data.bin
  sev-snp-report.bin
  stats.json
  job.json
  job.prepared.json
  job.prepared.pretty.json
  run.stdout
  verify-bhtm-quote.txt
  amd-snp-verify.txt

/root/blockhash-to-muhash/leaf-proofs/mainnet-900000-954920/
  height-948454.leaf-proof.json
  height-948454.verify.txt
  height-948454.run.log
```

Current HTTPS proof endpoint for BHTM chunk artifacts:

```text
https://65-21-91-217.sslip.io/mainnet-900000-954920/proof-generic/
```

Important: the endpoint supports `GET`, but `HEAD` currently returns `501`.
Do not use `HEAD` as the reachability test. For example, this works:

```bash
curl -fsS https://65-21-91-217.sslip.io/mainnet-900000-954920/proof-generic/stats.json
curl -fsS https://65-21-91-217.sslip.io/mainnet-900000-954920/proof-generic/verify-bhtm-quote.txt
```

The leaf proof is currently archived on `pir-hetzner`; if it is not already
served by HTTPS when you begin, publish or mirror it to a stable static path
before wiring the browser. Do not make the browser depend on SSH, `/tmp`, or
operator-local paths.

## Implementation Goals

Implement three things:

1. **Artifact loading**
   - Add a stable browser-loadable proof manifest for the current witness.
   - Include URLs or static paths for:
     - BHTM leaf proof JSON;
     - BHTM `attestation.bin`;
     - BHTM `report-data.bin`;
     - BHTM `sev-snp-report.bin`;
     - BHTM `stats.json`;
     - BHTM `job.json`;
     - DB proof bundle or live DB proof source already used by the frontend.
   - Prefer a small versioned manifest, e.g.
     `proofs/trust-chain/delta_940611_948454.json`, so future anchors can be
     added without rewriting UI logic.

2. **Browser verification**
   - Verify the existing DB proof as the frontend already does.
   - Verify the BHTM leaf proof against the committed BHTM tree root.
   - Verify BHTM `report-data.bin` equals:
     `SHA512("BitcoinPIR/blockhash-to-muhash/report-data/v2" || attestation.bin)`.
   - Verify the raw SEV-SNP `REPORT_DATA` field in `sev-snp-report.bin` matches
     `report-data.bin`.
   - Verify the raw SEV-SNP `MEASUREMENT` field matches the pinned BHTM streaming
     UKI measurement above.
   - Verify the AMD VCEK/report signature using the existing
     `pir-attest-verify` path, not a hand-rolled ASN.1/signature verifier.
   - Chain DB proof to BHTM proof by requiring:
     - DB proof `height == 948454`;
     - DB proof `blockHashHex == BHTM leaf block_hash`;
     - DB proof `muhashHex == BHTM leaf core_muhash`;
     - DB proof bucket/onion roots match `PRODUCTION_DB_PROOF_PINS`;
     - BHTM leaf `tree_root == BHTM attestation tree_root`.

3. **User-facing reproducibility UI**
   - Update `/Users/cusgadmin/BitcoinPIR/web/reproduce.html`.
   - Show a concise "Database authenticity chain" section:
     - DB proof verified;
     - BHTM leaf proof verified;
     - BHTM SEV-SNP quote verified;
     - shared anchor height/block hash/MuHash;
     - database roots.
   - Update the existing DB proof badge or add an adjacent "Bitcoin anchor"
     badge so users can distinguish:
     - runtime server attestation;
     - DB build proof;
     - Bitcoin/MuHash anchor proof.
   - Avoid overclaiming. The correct claim is:
     "attested MuHash transition from Core-style deltas", not
     "full Bitcoin consensus validation inside the TEE".

## Exact BHTM Attestation v2 Layout

The BHTM `attestation.bin` is little-endian, version 2:

```text
offset  size  field
0       2     version = 2
2       2     job_version
4       4     start_height
8       4     end_height
12      32    start_block_hash
44      32    end_block_hash
76      32    job_sha256
108     32    tree_root
140     8     tree_size
148     384   start_muhash
532     384   end_muhash
916 total
```

The SEV-SNP raw report offsets currently used by the project are:

```text
REPORT_DATA: offset 0x50, length 64
MEASUREMENT: offset 0x90, length 48
```

Port the exact BHTM leaf hashing and Merkle proof verification from:

```text
/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash/src/stream.rs
/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash/src/merkle_tree.rs
```

Do not guess byte order. The leaf proof JSON includes both internal/display
forms where needed; preserve the existing CLI semantics.

## Website Update

For the educational website, add a new section after the current verification
chapter, or add one new page under the existing verification chapter if you want
a smaller change.

Use this source draft:

```text
/Users/cusgadmin/bitcoin-pir/blockhash-to-muhash/docs/WEBSITE_DATABASE_AUTHENTICITY_SUMMARY.md
```

Suggested section:

```text
14. How we know the database matches Bitcoin
  14.1 The authenticity chain
  14.2 Inside the MuHash proof
  14.3 What is trusted, and what is not
```

The website should explain, visually if possible:

```text
Wallet query result
  -> Merkle proof to database root
  -> database bucket/onion super-roots
  -> attested database build proof
  -> snapshot anchor: height, block hash, MuHash
  -> BHTM leaf proof
  -> BHTM chunk tree root
  -> SEV-SNP quote over generic BHTM image
```

Keep style consistent with the existing Astro guide:

- add components under `src/components/sections/`;
- register pages in `src/data/sections.ts`;
- update `CONTENT-AUDIT.md` with source citations;
- run the website build/test commands from `package.json`;
- visually check desktop and mobile if you add layout/diagram changes.

## Frontend Update

For `/Users/cusgadmin/BitcoinPIR/web`, build on the existing DB proof surface.

Important existing files:

```text
web/src/db-proof.ts
web/src/attest-pin.ts
web/src/sdk-bridge.ts
web/index.html
web/reproduce.html
```

Likely implementation shape:

- add a `web/src/trust-chain-proof.ts` module;
- add a static proof manifest/pin in `web/src/attest-pin.ts` or a nearby module;
- add tests for:
  - parsing BHTM attestation v2;
  - report-data binding;
  - SEV report offset extraction;
  - BHTM leaf proof verification;
  - chain equality between DB proof anchor and BHTM leaf;
- expose a small status object to the UI:

```ts
type BitcoinAnchorProofStatus =
  | { state: 'not-checked' }
  | { state: 'verified'; height: number; blockHashHex: string; muhashHex: string; treeRootHex: string }
  | { state: 'unverified'; error: string; mismatches?: string[] }
  | { state: 'unavailable'; error: string };
```

If browser-side AMD VCEK verification is not currently exported from WASM for
static BHTM reports, add the minimal WASM export from the existing
`pir-attest-verify` code. Do not treat `verify-bhtm-quote.txt` as sufficient
cryptographic verification; it is useful for display/debug but the browser must
verify raw artifacts itself if the UI says "verified".

## Security Wording

Use this public wording:

> The attested MuHash runner checks the chain of block headers and the txid
> Merkle root commitments, then applies Core-format UTXO deltas. It is a compact
> computation proof for the MuHash transition, not a full Bitcoin consensus
> validator.

Do not claim:

- "full Bitcoin consensus validation in the TEE";
- "the TEE validates scripts/signatures";
- "the TEE independently reconstructs the UTXO set";
- "duplicate txid/BIP30 handling is rediscovered by the BHTM builder".

The data source/extractor is responsible for Bitcoin Core-compatible coin
deltas, including undo-data behavior, duplicate txids, genesis handling, and
unspendable-output filtering.

## Acceptance Criteria

Website:

- The public guide has a clear database-authenticity section/page.
- The new copy cites local sources in `CONTENT-AUDIT.md`.
- It explicitly distinguishes privacy verification from database authenticity.
- It states the BHTM trust boundary honestly.
- Site build succeeds.

Frontend:

- Browser can load the current trust-chain proof manifest.
- Browser verifies the DB proof and BHTM proof from raw artifacts.
- Browser chains the DB anchor to the BHTM leaf.
- Reproducibility page shows the current anchor:
  height `948454`, block hash, MuHash, DB roots, BHTM tree root, and BHTM UKI
  measurement.
- UI distinguishes runtime attestation, DB build proof, and Bitcoin anchor proof.
- Tests cover success and mismatch cases.
- No frontend status marked "verified" is based only on precomputed text files
  such as `verify-bhtm-quote.txt`.

## Useful Commands

In `blockhash-to-muhash`:

```bash
cargo test
cargo build --release
target/release/blockhash-to-muhash verify-leaf-proof \
  /tmp/bhtm-leaf-mainnet-900000-954920-h948454.json \
  --expect-height 948454 \
  --expect-tree-root babeea635812c3b1a2d5f352ab0a5d1ee8a4e9c668c43c05d6603ef3c3766ba6
```

In `BitcoinPIR`, inspect existing DB proof verification:

```bash
rg -n "db-proof|PRODUCTION_DB_PROOF_PINS|verify_database_proof|reproduce" web pir-sdk-client pir-sdk-wasm bpir-admin pir-attest-verify
```

In `website`, inspect and build:

```bash
npm run build
npm run check-audit-links
```

Confirm exact package scripts before running commands; do not assume script
names if `package.json` differs.
