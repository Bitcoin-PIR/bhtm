#!/usr/bin/env bash
# Submit a canonical BHTM runtime job to the generic proof UKI and verify outputs.

set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "Usage: $0 <job.json> [output-dir]" >&2
    echo >&2
    echo "Env:" >&2
    echo "  BHTM_BINARY       blockhash-to-muhash binary (default: target/release/blockhash-to-muhash)" >&2
    echo "  VPSBG_JOB_URL     generic UKI job endpoint (default: http://87.120.8.198:8080/job)" >&2
    echo "  SUBMIT_TIMEOUT    seconds to retry job submission (default: 600)" >&2
    echo "  PROOF_TIMEOUT     seconds to wait for uploaded proof files (default: 1800)" >&2
    exit 2
fi

JOB_IN=$1
OUT_DIR=${2:-}
BHTM_BINARY=${BHTM_BINARY:-target/release/blockhash-to-muhash}
VPSBG_JOB_URL=${VPSBG_JOB_URL:-http://87.120.8.198:8080/job}
SUBMIT_TIMEOUT=${SUBMIT_TIMEOUT:-600}
PROOF_TIMEOUT=${PROOF_TIMEOUT:-1800}

[ -x "$BHTM_BINARY" ] || {
    echo "error: BHTM_BINARY is not executable: $BHTM_BINARY" >&2
    exit 1
}
[ -r "$JOB_IN" ] || {
    echo "error: job file is not readable: $JOB_IN" >&2
    exit 1
}

TMP_DIR=$(mktemp -d /tmp/bhtm-submit.XXXXXX)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

JOB_CANON="$TMP_DIR/job.json"
"$BHTM_BINARY" canonical-job "$JOB_IN" > "$JOB_CANON"
"$BHTM_BINARY" job-env "$JOB_CANON" > "$TMP_DIR/job.env"
# shellcheck disable=SC1090
. "$TMP_DIR/job.env"

if [ -z "$OUT_DIR" ]; then
    OUT_DIR="/tmp/bhtm-proof-${BHTM_CHUNK}"
fi
mkdir -p "$OUT_DIR"
cp "$JOB_CANON" "$OUT_DIR/job.submitted.json"

echo "job_sha256=$BHTM_JOB_SHA256"
echo "chunk=$BHTM_CHUNK"
echo "upload_url=$BHTM_UPLOAD_URL"
echo "vpsbg_job_url=$VPSBG_JOB_URL"

deadline=$((SECONDS + SUBMIT_TIMEOUT))
submitted=0
while [ "$SECONDS" -lt "$deadline" ]; do
    if curl -fsS --connect-timeout 5 --max-time 15 \
        --request PUT --data-binary @"$JOB_CANON" "$VPSBG_JOB_URL" \
        >"$TMP_DIR/submit.out" 2>"$TMP_DIR/submit.err"; then
        submitted=1
        break
    fi
    sleep 2
done

if [ "$submitted" -ne 1 ]; then
    echo "error: failed to submit job within ${SUBMIT_TIMEOUT}s" >&2
    if [ -s "$TMP_DIR/submit.err" ]; then
        cat "$TMP_DIR/submit.err" >&2
    fi
    exit 1
fi
echo "job submitted"

deadline=$((SECONDS + PROOF_TIMEOUT))
files="job.json attestation.bin report-data.bin sev-snp-report.bin stats.json run.stdout"
while [ "$SECONDS" -lt "$deadline" ]; do
    all=1
    for f in $files; do
        if ! curl -fsS --connect-timeout 5 --max-time 30 \
            -o "$OUT_DIR/$f.tmp" "$BHTM_UPLOAD_URL/$f" \
            >"$TMP_DIR/fetch.out" 2>"$TMP_DIR/fetch.err"; then
            all=0
            rm -f "$OUT_DIR/$f.tmp"
            break
        fi
        mv "$OUT_DIR/$f.tmp" "$OUT_DIR/$f"
    done
    if [ "$all" -eq 1 ]; then
        break
    fi
    sleep 5
done

missing=0
for f in $files; do
    if [ ! -s "$OUT_DIR/$f" ]; then
        echo "missing proof artifact: $OUT_DIR/$f" >&2
        missing=1
    fi
done
if [ "$missing" -ne 0 ] && [ -s "$TMP_DIR/fetch.err" ]; then
    cat "$TMP_DIR/fetch.err" >&2
fi
if [ "$missing" -ne 0 ]; then
    exit 1
fi

"$BHTM_BINARY" verify-bhtm-quote \
    "$OUT_DIR/attestation.bin" \
    "$OUT_DIR/report-data.bin" \
    "$OUT_DIR/sev-snp-report.bin" \
    | tee "$OUT_DIR/verify-bhtm-quote.txt"

grep -q '^attestation_version=2$' "$OUT_DIR/verify-bhtm-quote.txt" || {
    echo "error: expected attestation_version=2" >&2
    exit 1
}
grep -q "^job_sha256=$BHTM_JOB_SHA256$" "$OUT_DIR/verify-bhtm-quote.txt" || {
    echo "error: verified job_sha256 does not match submitted job" >&2
    exit 1
}

measurement=$(dd if="$OUT_DIR/sev-snp-report.bin" bs=1 skip=$((0x90)) count=48 2>/dev/null | xxd -p -c 256)
report_data=$(dd if="$OUT_DIR/sev-snp-report.bin" bs=1 skip=$((0x50)) count=64 2>/dev/null | xxd -p -c 256)
{
    echo "measurement=$measurement"
    echo "report_data=$report_data"
    echo "job_sha256=$BHTM_JOB_SHA256"
    echo "upload_url=$BHTM_UPLOAD_URL"
    echo "output_dir=$OUT_DIR"
} | tee "$OUT_DIR/proof-summary.env"

echo "proof verified: $OUT_DIR"
