#!/bin/bash
# dracut module-setup for bhtm-proof-init.
#
# This module follows the same verified dracut shape as
# `97bpir-tier3-init/module-setup.sh`, but installs the one-shot
# proof init instead of the long-lived runit service tree. The kernel
# cmdline uses `rdinit=/sbin/bhtm-proof-init`, so dracut's own /init is
# bypassed at boot; dracut is still used here to assemble a complete
# initramfs userspace closure.
#
# Required kernel modules (already baked into the VPSBG kernel tree):
#   ccp, sev-guest, tsm_report — loaded by bhtm-proof-init.sh.
# Required tools:
#   curl, wget, sha256sum, zstd, ip, modprobe, mount, poweroff, sleep, python3.
# Required binaries:
#   /usr/local/bin/blockhash-to-muhash — the actual verifier.

# shellcheck shell=bash

check() {
    for b in busybox zstd curl wget ip modprobe python3; do
        if ! command -v "$b" >/dev/null 2>&1; then
            derror "bhtm-proof-init: $b not in \$PATH on build host"
            return 1
        fi
    done
    if [ ! -x /usr/local/bin/blockhash-to-muhash ]; then
        derror "bhtm-proof-init: /usr/local/bin/blockhash-to-muhash missing"
        derror "  build it: cargo build --release (in blockhash-to-muhash/)"
        return 1
    fi
    if ! python3 - <<'PY' >/dev/null 2>&1
import http.server
import socketserver
PY
    then
        derror "bhtm-proof-init: python3 cannot import http.server/socketserver"
        return 1
    fi
    return 0
}

depends() {
    echo "busybox base network"
    return 0
}

install() {
    # Tools used by the init script.
    inst_multiple ip modprobe mount sleep ln mkdir cat sh curl wget \
                  sha256sum zstd poweroff reboot sync tail tee awk \
                  stat chmod ls python3

    # udhcpc fallback for environments where static networking is not
    # supplied. VPSBG production proofs use static bhtm.* cmdline
    # parameters, but keeping the fallback costs little and helps
    # non-production smoke tests.
    if [ -x /usr/bin/busybox ]; then
        inst_simple /usr/bin/busybox
        ln_r /usr/bin/busybox /sbin/udhcpc
    else
        derror "bhtm-proof-init: /usr/bin/busybox not found"
        derror "  install with: apt install busybox-static"
        return 1
    fi

    # HTTPS verification for wget. Ubuntu's ca-certificates package
    # provides this bundle.
    if [ -r /etc/ssl/certs/ca-certificates.crt ]; then
        inst_simple /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
    else
        dwarn "bhtm-proof-init: /etc/ssl/certs/ca-certificates.crt missing; HTTPS may fail"
    fi

    # blockhash-to-muhash: the trust-bearing binary.
    inst /usr/local/bin/blockhash-to-muhash

    # Python's imports are dynamic enough that dracut will not discover
    # the stdlib closure from /usr/bin/python3 alone. Copy the stdlib
    # explicitly so the one-shot HTTP job receiver can import
    # http.server inside the initramfs.
    py_stdlib=$(python3 - <<'PY'
import sysconfig
print(sysconfig.get_paths()["stdlib"])
PY
)
    if [ -d "$py_stdlib" ]; then
        mkdir -p "$initdir$(dirname "$py_stdlib")"
        cp -a "$py_stdlib" "$initdir$(dirname "$py_stdlib")/"
        find "$initdir$py_stdlib" -type d -name __pycache__ -prune -exec rm -rf {} + 2>/dev/null || true
        rm -rf "$initdir$py_stdlib/test" "$initdir$py_stdlib/ensurepip" 2>/dev/null || true
    else
        derror "bhtm-proof-init: python stdlib not found: $py_stdlib"
        return 1
    fi

    # Our own init script (rdinit= target).
    inst_simple "$moddir/bhtm-proof-init.sh" /sbin/bhtm-proof-init
    inst_simple "$moddir/bhtm-job-server.py" /sbin/bhtm-job-server.py
}
