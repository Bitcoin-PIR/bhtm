#!/usr/bin/env bash
# Operator runbook for one blockhash-to-muhash chunk using the generic
# SEV-SNP proof-runner UKI.
#
# The UKI is built once and reused. Chunk-specific values are written to
# runtime job.json and bound into attestation v2 by job_sha256.
#
# Usage:
#
#   ./scripts/bhtm_proof_one_chunk.sh <chunk-name> \
#       <chunk-url> <chunk-stream-sha256> <attest-url> \
#       <expected-end-core-muhash> \
#       [<start-height> <start-block-hash> <start-muhash>]

set -euo pipefail

if [ "$#" -ne 5 ] && [ "$#" -ne 8 ]; then
    echo "Usage: $0 <chunk-name> <chunk-url> <chunk-stream-sha256> <attest-url> <expected-end-core-muhash> [<start-height> <start-block-hash> <start-muhash>]" >&2
    exit 1
fi

CHUNK_NAME=$1
CHUNK_URL=$2
CHUNK_STREAM_SHA256=$3
ATTEST_URL=$4
EXPECTED_END_CORE_MUHASH=$5
START_HEIGHT=${6:-}
START_BLOCK_HASH=${7:-}
START_MUHASH=${8:-}

CHAIN=${CHAIN:-mainnet}
VPSBG_HOST=${VPSBG_HOST:-vpsbg-pir}
VPSBG_JOB_URL=${VPSBG_JOB_URL:-http://87.120.8.198:8080/job}
JOB_PATH=${JOB_PATH:-/tmp/bhtm-$CHUNK_NAME.job.json}

echo "==================================================================="
echo "Generic BHTM SEV-SNP proof job"
echo "==================================================================="
echo "  chunk:                $CHUNK_NAME"
echo "  chain:                $CHAIN"
echo "  chunk_url:            $CHUNK_URL"
echo "  stream_sha256:        $CHUNK_STREAM_SHA256"
echo "  attest_url:           $ATTEST_URL"
echo "  end_core_muhash:      $EXPECTED_END_CORE_MUHASH"
[ -n "$START_HEIGHT" ] && echo "  start_height:         $START_HEIGHT"
[ -n "$START_BLOCK_HASH" ] && echo "  start_block_hash:     $START_BLOCK_HASH"
echo "  job file:             $JOB_PATH"
echo "  job submit URL:       $VPSBG_JOB_URL"
echo

mkdir -p "$(dirname "$JOB_PATH")"
if [ -n "$START_HEIGHT" ]; then
    cat > "$JOB_PATH" <<JOB
{
  "version": 1,
  "chain": "$CHAIN",
  "chunk": "$CHUNK_NAME",
  "input_url": "$CHUNK_URL",
  "input_sha256": "$CHUNK_STREAM_SHA256",
  "upload_url": "$ATTEST_URL",
  "expected_end_core_muhash": "$EXPECTED_END_CORE_MUHASH",
  "start_anchor": {
    "kind": "trusted_block",
    "height": $START_HEIGHT,
    "block_hash": "$START_BLOCK_HASH",
    "muhash": "$START_MUHASH"
  }
}
JOB
else
    cat > "$JOB_PATH" <<JOB
{
  "version": 1,
  "chain": "$CHAIN",
  "chunk": "$CHUNK_NAME",
  "input_url": "$CHUNK_URL",
  "input_sha256": "$CHUNK_STREAM_SHA256",
  "upload_url": "$ATTEST_URL",
  "expected_end_core_muhash": "$EXPECTED_END_CORE_MUHASH",
  "start_anchor": { "kind": "before_genesis" }
}
JOB
fi

echo "[Step 0/5] Validate and canonicalize job locally"
echo "  blockhash-to-muhash canonical-job '$JOB_PATH' > '$JOB_PATH.canonical'"
echo "  blockhash-to-muhash job-env '$JOB_PATH.canonical'"
echo

echo "[Step 1/5] Build the generic UKI on $VPSBG_HOST, if not already built"
echo "  ssh $VPSBG_HOST KERNEL=/boot/vmlinuz-7.0.0-15-generic \\"
echo "      BLOCKHASH_TO_MUHASH_BIN=/root/blockhash-to-muhash-proof/target/release/blockhash-to-muhash \\"
echo "      BHTM_STATIC_IP=87.120.8.198/32 BHTM_GATEWAY=172.16.0.1 BHTM_DNS=9.9.9.9 \\"
echo "      OUT=/tmp/bhtm-proof-generic.efi \\"
echo "      '/root/bhtm/scripts/build_bhtm_proof_uki.sh'"
echo

echo "[Step 2/5] Upload/select /tmp/bhtm-proof-generic.efi in the VPSBG portal"
echo "  Reusing the same UKI across chunks should preserve the SEV-SNP MEASUREMENT."
echo

echo "[Step 3/5] Reboot VPSBG, then submit the runtime job"
echo "  curl -fS --request PUT --data-binary '@$JOB_PATH' '$VPSBG_JOB_URL'"
echo

echo "[Step 4/5] Wait for the guest to power off after uploading proof artifacts"
echo "  Expected uploads:"
echo "    job.json, attestation.bin, report-data.bin, sev-snp-report.bin, stats.json, run.stdout"
echo

echo "[Step 5/5] Verify"
echo "  blockhash-to-muhash verify-bhtm-quote \\"
echo "      <proof>/attestation.bin \\"
echo "      <proof>/report-data.bin \\"
echo "      <proof>/sev-snp-report.bin"
echo "  Confirm:"
echo "    attestation_version=2"
echo "    job_sha256 matches: blockhash-to-muhash job-env <proof>/job.json"
echo "    end_muhash_core_display in stats.json matches the expected Core MuHash"
echo "    the SEV-SNP MEASUREMENT matches the approved generic UKI measurement"
