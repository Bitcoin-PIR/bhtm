#!/usr/bin/env bash
# Build a generic SEV-SNP UKI for the blockhash-to-muhash chunk
# archive proof runner.
#
# This is the imperative, operator-facing build path. It mirrors the
# verified Tier 3 UKI pipeline: use dracut to assemble a complete
# initramfs userspace closure, then use objcopy to assemble the PE/UKI
# sections.
#
# Differences vs scripts/build_uki_tier3.sh:
# - The bundled binary is `blockhash-to-muhash` (not `unified_server`).
# - The init script is `bhtm-proof-init` (PID-1 one-shot job runner),
#   NOT `bpir-tier3-init` (long-lived service supervisor).
# - The cmdline is intentionally chunk-neutral. Chunk URL, stream hash,
#   upload URL, anchors, and expected MuHash live in runtime job.json
#   and are bound into attestation v2 by job_sha256.
#
# Operator usage (must run as root — /boot/vmlinuz-* is mode 0600):
#
#   # 1. Build blockhash-to-muhash once (release binary, pinned toolchain).
#   cd ~/bitcoin-pir/blockhash-to-muhash
#   cargo build --release
#   # Verify the SHA matches the operator-published pin:
#   sha256sum target/release/blockhash-to-muhash
#
#   # 2. Build the generic UKI. Required env:
#   ssh vpsbg-pir 'BLOCKHASH_TO_MUHASH_BIN=~/bitcoin-pir/blockhash-to-muhash/target/release/blockhash-to-muhash \
#       BHTM_STATIC_IP=87.120.8.198/32 BHTM_GATEWAY=172.16.0.1 BHTM_DNS=9.9.9.9 \
#       ~/bhtm/scripts/build_bhtm_proof_uki.sh'
#
#   # 3. Recover from a failed boot:
#   VPSBG portal → Measured Boot → UKI → set to "None" → reboot →
#   stock Ubuntu rootfs comes up with sshd.
#
# The output UKI is at /tmp/bhtm-proof-generic.efi. Capture its sha256
# along with the cmdline for the published MRTD chain.

set -euo pipefail

if [ "$EUID" != "0" ]; then
    echo "error: build_bhtm_proof_uki.sh must run as root — /boot/vmlinuz-* is" >&2
    echo "       not readable by the pir user. Re-run as:" >&2
    echo "         ssh vpsbg-pir '~/bhtm/scripts/build_bhtm_proof_uki.sh'" >&2
    exit 1
fi

# ─── Required inputs ──────────────────────────────────────────────
: "${BLOCKHASH_TO_MUHASH_BIN:?BLOCKHASH_TO_MUHASH_BIN must point to a release binary}"

# ─── Defaults ─────────────────────────────────────────────────────
OUT=${OUT:-/tmp/bhtm-proof-generic.efi}
KERNEL=${KERNEL:-}
BHTM_JOB_PORT=${BHTM_JOB_PORT:-8080}

# Resolve dracut module dir relative to this script (kept for parity
# with build_uki_tier3.sh, even though the nix path is preferred).
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

# ─── Sanity checks ────────────────────────────────────────────────
for tool in objcopy objdump dracut lsinitrd sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "error: $tool not installed (apt install binutils coreutils dracut)" >&2
        exit 1
    }
done

# Build-host must have blockhash-to-muhash at the requested path.
[ -x "$BLOCKHASH_TO_MUHASH_BIN" ] || {
    echo "error: BLOCKHASH_TO_MUHASH_BIN=$BLOCKHASH_TO_MUHASH_BIN is not executable" >&2
    echo "       run: cargo build --release (in blockhash-to-muhash/)" >&2
    exit 1
}

# ─── Kernel auto-detection ───────────────────────────────────────
if [ -z "$KERNEL" ]; then
    KERNEL=$(ls -1 /boot/vmlinuz-*-generic 2>/dev/null | sort -V | tail -1)
    if [ -z "$KERNEL" ]; then
        echo "error: no kernel found under /boot/vmlinuz-*-generic — set KERNEL= explicitly" >&2
        exit 1
    fi
    echo "auto-detected kernel: $KERNEL"
fi
[ -r "$KERNEL" ] || { echo "error: $KERNEL not readable" >&2; exit 1; }

KVER=$(basename "$KERNEL" | sed 's/^vmlinuz-//')
[ -d "/usr/lib/modules/$KVER" ] || {
    echo "error: /usr/lib/modules/$KVER missing — kernel modules not installed?" >&2
    exit 1
}

# Validate SEV-SNP kernel modules exist BEFORE building the initramfs.
SEV_MODULES_DIR="/usr/lib/modules/$KVER/kernel/drivers"
REQUIRED_SEV_MODS="ccp sev-guest tsm_report"
MISSING_REQUIRED=""
for mod in $REQUIRED_SEV_MODS; do
    if ! find "$SEV_MODULES_DIR" -name "$mod.ko*" -print -quit 2>/dev/null | grep -q .; then
        MISSING_REQUIRED="$MISSING_REQUIRED $mod"
    fi
done
if [ -n "$MISSING_REQUIRED" ]; then
    echo "error: SEV kernel module(s) not found under $SEV_MODULES_DIR:$MISSING_REQUIRED" >&2
    echo "       (the VPSBG kernel is required for SEV-SNP attestation)" >&2
    exit 1
fi
echo "SEV modules: $REQUIRED_SEV_MODS — all found in $SEV_MODULES_DIR"

# ─── Install dracut module + trust-bearing binary ─────────────────
DRACUT_MODULE_DIR="$SCRIPT_DIR/dracut"
BHTM_MODULE_SRC="$DRACUT_MODULE_DIR/98bhtm-proof-init"
BHTM_MODULE_DST="/usr/lib/dracut/modules.d/98bhtm-proof-init"
[ -d "$BHTM_MODULE_SRC" ] || { echo "error: dracut module missing: $BHTM_MODULE_SRC" >&2; exit 1; }

mkdir -p "$BHTM_MODULE_DST"
find "$BHTM_MODULE_SRC" -maxdepth 1 -type f -exec cp -fp {} "$BHTM_MODULE_DST/" \;
chmod 0755 "$BHTM_MODULE_DST"/*
find "$BHTM_MODULE_DST" -type f -exec touch -d @0 {} +
echo "dracut module installed: $BHTM_MODULE_DST"

install -m0755 "$BLOCKHASH_TO_MUHASH_BIN" /usr/local/bin/blockhash-to-muhash
touch -d @0 /usr/local/bin/blockhash-to-muhash || true
BHTM_BIN_HASH=$(sha256sum /usr/local/bin/blockhash-to-muhash | awk '{print $1}')
echo "blockhash-to-muhash installed: /usr/local/bin/blockhash-to-muhash"
echo "binary sha256: $BHTM_BIN_HASH"

# ─── Build the initramfs ─────────────────────────────────────────
INITRD=/tmp/bhtm-proof-initrd.img
DRIVER_LIST="virtio_net virtio_pci $REQUIRED_SEV_MODS"

echo "==> Building dracut initramfs at $INITRD"
SOURCE_DATE_EPOCH=0 dracut --force --no-hostonly --reproducible --nostrip \
    --add "network bhtm-proof-init" \
    --add-drivers " $DRIVER_LIST " \
    --kver "$KVER" \
    "$INITRD"
echo "  initrd: $(du -h "$INITRD" | cut -f1)"

echo "==> Validating initramfs contents"
INITRD_LISTING=$(lsinitrd "$INITRD" 2>/dev/null)
if [ -z "$INITRD_LISTING" ]; then
    echo "error: lsinitrd failed — cannot inspect initramfs" >&2
    exit 1
fi

require_initrd_entry() {
    local label="$1"
    local pattern="$2"
    if ! grep -q -- "$pattern" <<< "$INITRD_LISTING"; then
        echo "error: initramfs missing $label ($pattern)" >&2
        exit 1
    fi
    echo "  ok: $label"
}

require_initrd_entry "bhtm proof init" "sbin/bhtm-proof-init"
require_initrd_entry "bhtm job server" "sbin/bhtm-job-server.py"
require_initrd_entry "blockhash-to-muhash binary" "usr/local/bin/blockhash-to-muhash"
for tool in sh ip modprobe curl wget zstd sha256sum python3; do
    require_initrd_entry "$tool" "/$tool"
done
require_initrd_entry "python http.server stdlib" "http/server.py"
for mod in $REQUIRED_SEV_MODS; do
    require_initrd_entry "$mod module" "${mod}.ko"
done

# ─── Assemble the UKI ────────────────────────────────────────────
echo "==> Assembling UKI at $OUT"
STUB=$(find /usr/lib/systemd/boot/efi -name linuxx64.efi.stub 2>/dev/null | head -1)
if [ -z "$STUB" ]; then
    echo "error: linuxx64.efi.stub not found under /usr/lib/systemd/boot/efi" >&2
    echo "       apt install systemd-boot" >&2
    exit 1
fi

# Build the cmdline. Keep it chunk-neutral so the SEV-SNP MEASUREMENT
# can be reused across all chunk jobs. Optional bhtm.job_url and
# bhtm.job_sha256 are provided only for automation/smoke runs; using
# chunk-specific values there intentionally changes the measurement.
printf '%s' \
    "rdinit=/sbin/bhtm-proof-init console=ttyS0,115200 console=tty1 quiet loglevel=3 \
bhtm.job_port=$BHTM_JOB_PORT" \
    > /tmp/bhtm-proof.cmdline
[ -n "${BHTM_JOB_URL:-}" ]                  && printf ' bhtm.job_url=%s'                  "$BHTM_JOB_URL"                  >> /tmp/bhtm-proof.cmdline
[ -n "${BHTM_JOB_SHA256:-}" ]               && printf ' bhtm.job_sha256=%s'               "$BHTM_JOB_SHA256"               >> /tmp/bhtm-proof.cmdline
[ -n "${BHTM_STATIC_IP:-}" ]                && printf ' bhtm.static_ip=%s'                "$BHTM_STATIC_IP"                >> /tmp/bhtm-proof.cmdline
[ -n "${BHTM_GATEWAY:-}" ]                  && printf ' bhtm.gateway=%s'                  "$BHTM_GATEWAY"                  >> /tmp/bhtm-proof.cmdline
[ -n "${BHTM_DNS:-}" ]                      && printf ' bhtm.dns=%s'                      "$BHTM_DNS"                      >> /tmp/bhtm-proof.cmdline

printf 'NAME="bhtm"\nID=bhtm\nVERSION_ID="proof-v2-generic"\nPRETTY_NAME="BitcoinPIR blockhash-to-muhash generic proof runner (v2)"\n' \
    > /tmp/bhtm-proof.os-release

# PE section VMAs.
imagebase=$(( 0x$(objdump -p "$STUB" | awk '/^ImageBase/    {v=$2} END {print v}') ))
sizeofimage=$(( 0x$(objdump -p "$STUB" | awk '/^SizeOfImage/ {v=$2} END {print v}') ))
mib=$(( 1024 * 1024 ))
stub_end=$(( imagebase + sizeofimage ))
osrel_vma=$(( ( stub_end + mib - 1 ) / mib * mib ))
cmdline_vma=$(( osrel_vma   + mib ))
linux_vma=$((   cmdline_vma + mib ))
initrd_vma=$((  linux_vma + 64 * mib ))

linux_sz=$(stat -c %s "$KERNEL")
if [ "$linux_sz" -ge $(( 64 * mib )) ]; then
    echo "error: kernel ($linux_sz bytes) exceeds the 64 MiB .linux gap" >&2
    exit 1
fi

mkdir -p "$(dirname "$OUT")"
objcopy \
    --add-section .osrel=/tmp/bhtm-proof.os-release --change-section-vma .osrel="$osrel_vma" \
    --add-section .cmdline=/tmp/bhtm-proof.cmdline --change-section-vma .cmdline="$cmdline_vma" \
    --add-section .linux="$KERNEL"                  --change-section-vma .linux="$linux_vma" \
    --add-section .initrd="$INITRD"                 --change-section-vma .initrd="$initrd_vma" \
    "$STUB" "$OUT" 2> /tmp/bhtm-proof.objcopy.err \
    || { cat /tmp/bhtm-proof.objcopy.err >&2; echo "error: objcopy failed" >&2; exit 1; }

if [ -s /tmp/bhtm-proof.objcopy.err ]; then
    cat /tmp/bhtm-proof.objcopy.err
    if grep -q 'below image base' /tmp/bhtm-proof.objcopy.err; then
        echo "error: objcopy placed a section below the PE image base" >&2
        exit 1
    fi
fi

sections=$(objdump -h "$OUT")
echo "$sections" | grep -q -- ".osrel"   || { echo "error: .osrel section missing"   >&2; exit 1; }
echo "$sections" | grep -q -- ".cmdline" || { echo "error: .cmdline section missing" >&2; exit 1; }
echo "$sections" | grep -q -- ".linux"   || { echo "error: .linux section missing"   >&2; exit 1; }
echo "$sections" | grep -q -- ".initrd"  || { echo "error: .initrd section missing"  >&2; exit 1; }

out_soi=$(( 0x$(objdump -p "$OUT" | awk '/^SizeOfImage/ {v=$2} END {print v}') ))
initrd_sz=$(stat -c %s "$INITRD")
initrd_end_rva=$(( initrd_vma - imagebase + initrd_sz ))
if [ "$out_soi" -lt "$initrd_end_rva" ]; then
    echo "error: UKI SizeOfImage ($out_soi) does not cover .initrd end ($initrd_end_rva)" >&2
    exit 1
fi

echo "==> Built: $OUT"
echo "    sha256: $(sha256sum "$OUT" | awk '{print $1}')"
echo "    kernel: $KVER"
echo "    blockhash-to-muhash: $BHTM_BIN_HASH"
echo "    cmdline:"
sed 's/^/      /' /tmp/bhtm-proof.cmdline

echo
echo "Next steps:"
echo "  1. Capture the cmdline above + UKI sha256 + binary sha256 as the published MRTD chain."
echo "  2. Upload $OUT to the VPSBG Measured Boot portal."
echo "  3. Reboot the VPSBG VM. The init script will wait for a runtime job.json."
echo "  4. Submit a job, for example:"
echo "       curl -fS --request PUT --data-binary @job.json http://87.120.8.198:${BHTM_JOB_PORT}/job"
echo "  5. The guest will fetch the stream, run job-teed, upload proof artifacts,"
echo "     then power off."
echo "  6. Verify the uploaded artifacts with:"
echo "       blockhash-to-muhash verify-bhtm-quote \\"
echo "         <chunk>/proof/attestation.bin \\"
echo "         <chunk>/proof/report-data.bin \\"
echo "         <chunk>/proof/sev-snp-report.bin"
