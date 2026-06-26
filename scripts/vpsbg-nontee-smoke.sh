#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

VPSBG_HOST="${VPSBG_HOST:-vpsbg-pir}"
REMOTE_DIR="${REMOTE_DIR:-/root/blockhash-to-muhash-nontee}"
STREAM="${STREAM:-/tmp/bhtm_first_10000.bin}"
REMOTE_STREAM="${REMOTE_STREAM:-data/first_10000.bin}"
RUN_ID="${RUN_ID:-first-10000}"
EXPECTED_CORE_MUHASH="${EXPECTED_CORE_MUHASH:-fa8bc3bb3e3fb84e219da85b381cf140df1ee29d044272a6cac86e4a75c25cd4}"
SKIP_TESTS="${SKIP_TESTS:-0}"

if [[ ! -f "${STREAM}" ]]; then
  echo "missing stream: ${STREAM}" >&2
  exit 1
fi

echo "==> Staging source on ${VPSBG_HOST}:${REMOTE_DIR}"
rsync -az --delete \
  --exclude target \
  --exclude .git \
  --exclude data \
  --exclude proof-runs \
  "${REPO_DIR}/" "${VPSBG_HOST}:${REMOTE_DIR}/"

echo "==> Copying compact stream"
ssh "${VPSBG_HOST}" "mkdir -p '${REMOTE_DIR}/$(dirname "${REMOTE_STREAM}")'"
scp "${STREAM}" "${VPSBG_HOST}:${REMOTE_DIR}/${REMOTE_STREAM}"

echo "==> Remote input checksum"
ssh "${VPSBG_HOST}" "cd '${REMOTE_DIR}' && sha256sum '${REMOTE_STREAM}'"

if [[ "${SKIP_TESTS}" != "1" ]]; then
  echo "==> Running remote tests"
  ssh "${VPSBG_HOST}" "cd '${REMOTE_DIR}' && cargo test"
fi

echo "==> Building remote release binary"
ssh "${VPSBG_HOST}" "cd '${REMOTE_DIR}' && cargo build --release"

echo "==> Running non-TEE proof smoke"
ssh "${VPSBG_HOST}" "cd '${REMOTE_DIR}' && \
  rm -rf 'proof-runs/${RUN_ID}' && \
  mkdir -p 'proof-runs/${RUN_ID}' && \
  /usr/bin/time -v \
    target/release/blockhash-to-muhash \
      bin '${REMOTE_STREAM}' \
      --expect-end-core-muhash '${EXPECTED_CORE_MUHASH}' \
      --attestation-out 'proof-runs/${RUN_ID}/attestation.bin' \
      --report-data-out 'proof-runs/${RUN_ID}/report-data.bin' \
      --stats-json-out 'proof-runs/${RUN_ID}/stats.json' \
      > 'proof-runs/${RUN_ID}/run.stdout' \
      2> 'proof-runs/${RUN_ID}/run.stderr'"

echo "==> Remote proof artifacts"
ssh "${VPSBG_HOST}" "cd '${REMOTE_DIR}' && \
  ls -lh 'proof-runs/${RUN_ID}' && \
  sha256sum \
    'proof-runs/${RUN_ID}/attestation.bin' \
    'proof-runs/${RUN_ID}/report-data.bin' \
    'proof-runs/${RUN_ID}/stats.json' \
    'proof-runs/${RUN_ID}/run.stdout' \
    'proof-runs/${RUN_ID}/run.stderr' \
    target/release/blockhash-to-muhash && \
  python3 - <<'PY'
import json
with open('proof-runs/${RUN_ID}/stats.json') as f:
    data = json.load(f)
for key in [
    'end_height',
    'end_block_hash_display',
    'end_muhash_core_display',
    'tree_root',
    'attestation_size',
    'attestation_sha256',
    'tee_report_data',
]:
    print(f'{key}: {data[key]}')
PY"
