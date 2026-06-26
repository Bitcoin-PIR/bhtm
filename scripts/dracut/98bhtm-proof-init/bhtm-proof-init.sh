#!/bin/sh
# Generic PID 1 for blockhash-to-muhash SEV-SNP proofs.
#
# This initramfs is intentionally chunk-neutral. The UKI cmdline only
# carries generic boot/network controls plus optional job delivery
# controls. Chunk-specific URLs, stream hashes, anchors, and expected
# MuHash values are supplied at runtime in job.json and bound into the
# proof through attestation v2's job_sha256 field.

# shellcheck shell=sh

PATH=/usr/local/bin:/usr/bin:/usr/sbin:/sbin:/bin
export PATH

mount -t proc     proc     /proc     2>/dev/null || true
mount -t sysfs    sysfs    /sys      2>/dev/null || true
mount -t devtmpfs devtmpfs /dev      2>/dev/null || true
[ -d /dev/pts ] || mkdir -p /dev/pts
mount -t devpts   devpts   /dev/pts  2>/dev/null || true
[ -d /run ] || mkdir -p /run
mount -t tmpfs    tmpfs    /run      2>/dev/null || true
[ -d /tmp ] || mkdir -p /tmp
mount -t tmpfs    tmpfs    /tmp      2>/dev/null || true
mkdir -p /tmp/proof
chmod 700 /tmp/proof
: > /tmp/proof/run.stdout

log() {
    echo "[bhtm-proof-init] $*" | tee -a /tmp/proof/run.stdout
}

fatal() {
    log "FATAL: $*"
    sync
    sleep 2
    reboot -f
}

# Disable kernel console input vectors. The host can still force-stop
# or reset the VM through the hypervisor.
echo 0 > /proc/sys/kernel/sysrq        2>/dev/null || true
echo 0 > /proc/sys/kernel/ctrl-alt-del 2>/dev/null || true

CMDLINE=$(cat /proc/cmdline)
bhtm_static_ip=
bhtm_gateway=
bhtm_dns=
bhtm_job_url=
bhtm_job_sha256=
bhtm_job_port=8080

for kv in $CMDLINE; do
    case "$kv" in
        bhtm.static_ip=*) bhtm_static_ip="${kv#bhtm.static_ip=}" ;;
        bhtm.gateway=*) bhtm_gateway="${kv#bhtm.gateway=}" ;;
        bhtm.dns=*) bhtm_dns="${kv#bhtm.dns=}" ;;
        bhtm.job_url=*) bhtm_job_url="${kv#bhtm.job_url=}" ;;
        bhtm.job_sha256=*) bhtm_job_sha256="${kv#bhtm.job_sha256=}" ;;
        bhtm.job_port=*) bhtm_job_port="${kv#bhtm.job_port=}" ;;
        # rdinit=, console=, quiet, loglevel= — ignored.
    esac
done

log "generic proof runner starting"
log "job delivery: $([ -n "$bhtm_job_url" ] && echo fetch || echo listen)"
log "job port: $bhtm_job_port"

# Network bring-up.
modprobe virtio_net 2>/dev/null || true
modprobe virtio_pci 2>/dev/null || true
ip link set lo up 2>/dev/null || true

i=0
while [ ! -d /sys/class/net/eth0 ] && [ "$i" -lt 60 ]; do
    sleep 0.2
    i=$((i + 1))
done
[ -d /sys/class/net/eth0 ] || fatal "eth0 never appeared after 12s"

if [ -n "$bhtm_static_ip" ] && [ -n "$bhtm_gateway" ]; then
    ip link set eth0 up || fatal "failed to bring eth0 up"
    ip addr add "$bhtm_static_ip" dev eth0 || fatal "failed to add static IP"
    ip route add default via "$bhtm_gateway" dev eth0 onlink || fatal "failed to add default route"
    if [ -n "$bhtm_dns" ]; then
        mkdir -p /etc
        printf 'nameserver %s\n' "$bhtm_dns" > /etc/resolv.conf
    fi
else
    if [ -x /sbin/udhcpc ]; then
        /sbin/udhcpc -i eth0 -t 5 -n -q || fatal "udhcpc failed"
    else
        fatal "/sbin/udhcpc missing and no static IP supplied"
    fi
fi

ip addr show 2>&1 | tee -a /tmp/proof/run.stdout
ip route show 2>&1 | tee -a /tmp/proof/run.stdout

# SEV-SNP modules.
modprobe ccp        || log "WARN: modprobe ccp failed"
modprobe sev-guest  || log "WARN: modprobe sev-guest failed"
modprobe tsm_report || log "WARN: modprobe tsm_report failed"
[ -c /dev/sev-guest ] || fatal "/dev/sev-guest missing; refusing to continue"
ls -la /dev/sev-guest 2>&1 | tee -a /tmp/proof/run.stdout
log "/dev/sev-guest ready"

# Runtime job acquisition.
if [ -n "$bhtm_job_url" ]; then
    log "fetching job.json from $bhtm_job_url"
    curl -fL --connect-timeout 30 --retry 2 --retry-delay 2 \
        -o /tmp/job.raw.json "$bhtm_job_url" 2>&1 | tee -a /tmp/proof/run.stdout
else
    log "waiting for POST/PUT /job on 0.0.0.0:$bhtm_job_port"
    python3 /sbin/bhtm-job-server.py "$bhtm_job_port" /tmp/job.raw.json \
        2>&1 | tee -a /tmp/proof/run.stdout
fi
[ -s /tmp/job.raw.json ] || fatal "job.json was not received"

log "canonicalizing job.json"
blockhash-to-muhash canonical-job /tmp/job.raw.json > /tmp/proof/job.json \
    || fatal "job.json validation/canonicalization failed"
blockhash-to-muhash job-env /tmp/proof/job.json > /tmp/job.env \
    || fatal "failed to derive job environment"
# shellcheck disable=SC1091
. /tmp/job.env

if [ -n "$bhtm_job_sha256" ] && [ "$bhtm_job_sha256" != "$BHTM_JOB_SHA256" ]; then
    fatal "job SHA256 mismatch: expected $bhtm_job_sha256 got $BHTM_JOB_SHA256"
fi

log "job chain: $BHTM_CHAIN"
log "job chunk: $BHTM_CHUNK"
log "job sha256: $BHTM_JOB_SHA256"

# Fetch, hash, decompress, and process the compact stripped stream without
# materializing the compressed or decompressed inputs in tmpfs.
log "streaming stripped stream from $BHTM_INPUT_URL"
rm -f /tmp/chunk.zst.hashpipe /tmp/chunk.zst.sha
mkfifo /tmp/chunk.zst.hashpipe || fatal "failed to create hash fifo"
sha256sum /tmp/chunk.zst.hashpipe > /tmp/chunk.zst.sha &
hash_pid=$!

log "running blockhash-to-muhash job-teed"
cd /tmp || fatal "failed to cd /tmp"
set +e
curl -fL --connect-timeout 30 --retry 2 --retry-delay 2 "$BHTM_INPUT_URL" \
    2>> /tmp/proof/run.stdout \
    | tee /tmp/chunk.zst.hashpipe \
    | zstd -dc 2>> /tmp/proof/run.stdout \
    | blockhash-to-muhash job-teed /tmp/proof/job.json - \
    --attestation-out    /tmp/proof/attestation.bin \
    --report-data-out    /tmp/proof/report-data.bin \
    --stats-json-out     /tmp/proof/stats.json \
    --sev-snp-report-out /tmp/proof/sev-snp-report.bin \
    --job-out            /tmp/proof/job.json \
    --sev-missing fail \
    >> /tmp/proof/run.stdout 2>&1
rc=$?
wait "$hash_pid"
hash_rc=$?
set -e
[ "$hash_rc" -eq 0 ] || fatal "compressed stream sha256 worker failed"

got_sha=$(awk '{print $1}' /tmp/chunk.zst.sha)
if [ "$got_sha" != "$BHTM_INPUT_SHA256" ]; then
    fatal "stream SHA256 mismatch: expected $BHTM_INPUT_SHA256 got $got_sha"
fi
log "compressed stream sha256 ok: $got_sha"
[ "$rc" -eq 0 ] || fatal "job-teed exited $rc"

log "uploading proof artifacts to $BHTM_UPLOAD_URL"
upload_failed=0
for f in job.json attestation.bin report-data.bin sev-snp-report.bin stats.json run.stdout; do
    src=/tmp/proof/$f
    [ -f "$src" ] || continue
    dst="$BHTM_UPLOAD_URL/$f"
    log "PUT $dst"
    upload_log="/tmp/upload-$f.log"
    if curl -fS --connect-timeout 30 --upload-file "$src" "$dst" > "$upload_log" 2>&1; then
        cat "$upload_log" | tee -a /tmp/proof/run.stdout
    else
        cat "$upload_log" | tee -a /tmp/proof/run.stdout
        upload_failed=1
    fi
done
[ "$upload_failed" -eq 0 ] || fatal "one or more proof uploads failed"

log "proof artifact SHAs"
for f in job.json attestation.bin report-data.bin sev-snp-report.bin stats.json run.stdout; do
    if [ -f "/tmp/proof/$f" ]; then
        sha=$(sha256sum /tmp/proof/$f | awk '{print $1}')
        sz=$(stat -c %s /tmp/proof/$f)
        log "/tmp/proof/$f sha256=$sha size=$sz"
    fi
done

log "done; powering off"
sync
sleep 1
poweroff -f
