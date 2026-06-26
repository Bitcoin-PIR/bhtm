//! SEV-SNP guest-side attestation report fetcher.
//!
//! On a SEV-SNP guest, `/dev/sev-guest` exposes an ioctl that takes 64
//! bytes of attester-supplied "user data" (REPORT_DATA) and returns a
//! signed attestation report (~1184 bytes). The signature chains to the
//! VCEK certificate the AMD KDS issues for this exact chip + TCB, which
//! itself chains to AMD's ARK root.
//!
//! This module is structurally identical to
//! `BitcoinPIR/pir-runtime-core/src/attest.rs::fetch_report` and
//! `/home/pir/attested-builder/builder/src/evidence.rs::fetch_sev_snp_report`
//! — kept as a separate copy so that the `blockhash-to-muhash` crate
//! builds in isolation (its own `Cargo.lock`, no workspace dependency on
//! `pir-runtime-core`). The intent is that two operators running
//! `cargo build --release` on the same source + toolchain produce
//! byte-identical binaries, so the SEV-SNP MRTD pinned to this binary
//! is reproducible.
//!
//! On hosts without `/dev/sev-guest` (Hetzner's Intel i7-8700, dev
//! laptops, CI), [`fetch_report`] returns `Ok(None)` so the caller can
//! still emit the unsigned attestation + report-data triple.

use std::io;
use std::os::unix::io::AsRawFd;

/// Offset of the REPORT_DATA field inside a SEV-SNP attestation report.
///
/// Per AMD SEV-SNP firmware spec, the REPORT_DATA field is at offset
/// 0x50 in the raw report (1184-byte attestation report, v5+).
pub const SEV_SNP_REPORT_DATA_OFFSET: usize = 0x50;

/// Length in bytes of the REPORT_DATA field. Always 64.
pub const SEV_SNP_REPORT_DATA_LEN: usize = 64;

/// Length of a SEV-SNP attestation report (v5+).
pub const SEV_SNP_REPORT_LEN: usize = 1184;

/// Path the kernel exposes the SEV-SNP guest interface at.
#[cfg(unix)]
const SEV_GUEST_DEVICE: &str = "/dev/sev-guest";

// Linux uapi: include/uapi/linux/sev-guest.h (kernel 5.19+):
//
//   #define SNP_GUEST_REQ_IOC_TYPE 'S'         // = 0x53
//   struct snp_guest_request_ioctl {
//       __u8  msg_version;     // offset 0, +7 padding to 8
//       __u64 req_data;        // offset 8
//       __u64 resp_data;       // offset 16
//       __u64 exitinfo2;       // offset 24
//   };  // total 32 bytes on 64-bit Linux
//   #define SNP_GET_REPORT _IOWR(SNP_GUEST_REQ_IOC_TYPE, 0, struct snp_guest_request_ioctl)
//
// _IOWR encodes: dir=3 (R|W), size=32, type='S' (0x53), nr=0
//   = (3 << 30) | (32 << 16) | (0x53 << 8) | 0
//   = 0xc020_5300
#[cfg(target_env = "musl")]
const SNP_GET_REPORT_IOCTL: libc::c_int = 0xc020_5300u32 as libc::c_int;
#[cfg(not(target_env = "musl"))]
const SNP_GET_REPORT_IOCTL: libc::c_ulong = 0xc020_5300;

#[repr(C)]
struct SnpGuestRequestIoctl {
    msg_version: u8,
    _pad: [u8; 7],
    req_data: u64,
    resp_data: u64,
    exitinfo2: u64,
}

#[repr(C)]
struct SnpReportReq {
    user_data: [u8; SEV_SNP_REPORT_DATA_LEN],
    vmpl: u32,
    _rsvd: [u8; 28],
}

/// Response buffer per Linux `sev-guest.h` UAPI.
///
/// The kernel response begins with a 32-byte status/size/reserved
/// header, followed by the raw AMD SEV-SNP attestation report. The
/// report itself is what AMD signs and what VCEK verification consumes.
#[repr(C)]
struct SnpReportResp {
    status: u32,
    report_size: u32,
    _rsvd: [u8; 24],
    data: [u8; 4000],
}

/// Fetch a SEV-SNP attestation report binding `user_data` (64 bytes)
/// into the report's REPORT_DATA field.
///
/// Returns:
/// - `Ok(Some(report_bytes))` — raw signed attestation report bytes
///   (`SEV_SNP_REPORT_LEN` = 1184 for v5). The kernel response header
///   is stripped, so the caller gets the exact bytes AMD signed.
/// - `Ok(None)` — `/dev/sev-guest` doesn't exist (host isn't a SEV-SNP
///   guest). Caller decides whether to fail, warn, or silently emit
///   the unsigned attestation triple.
/// - `Err(_)` — unexpected I/O error (permissions, kernel bug,
///   malformed response).
pub fn fetch_report(user_data: [u8; SEV_SNP_REPORT_DATA_LEN]) -> io::Result<Option<Vec<u8>>> {
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(SEV_GUEST_DEVICE)
    {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };

    let mut req = SnpReportReq {
        user_data,
        vmpl: 0,
        _rsvd: [0u8; 28],
    };
    let mut resp = SnpReportResp {
        status: 0,
        report_size: 0,
        _rsvd: [0u8; 24],
        data: [0u8; 4000],
    };
    let mut wrap = SnpGuestRequestIoctl {
        msg_version: 1,
        _pad: [0u8; 7],
        req_data: &mut req as *mut _ as u64,
        resp_data: &mut resp as *mut _ as u64,
        exitinfo2: 0,
    };

    // SAFETY: `file` is a valid `/dev/sev-guest` fd; `wrap`, `req`,
    // and `resp` are properly-sized C-repr structs that outlive the
    // ioctl call. The kernel writes into `resp.data` and we read only
    // from `resp.data` after the ioctl returns.
    let rc = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            SNP_GET_REPORT_IOCTL,
            &mut wrap as *mut SnpGuestRequestIoctl,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }

    if resp.status != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("SEV-SNP report response status=0x{:x}", resp.status),
        ));
    }
    if (resp.report_size as usize) < SEV_SNP_REPORT_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SEV-SNP report too short: {} bytes (expected >= {})",
                resp.report_size, SEV_SNP_REPORT_LEN
            ),
        ));
    }
    Ok(Some(resp.data[..SEV_SNP_REPORT_LEN].to_vec()))
}

/// Extract the 64-byte REPORT_DATA field from a raw SEV-SNP
/// attestation report. Returns `None` if the report is too short to
/// contain the field.
pub fn extract_report_data(report: &[u8]) -> Option<[u8; SEV_SNP_REPORT_DATA_LEN]> {
    if report.len() < SEV_SNP_REPORT_DATA_OFFSET + SEV_SNP_REPORT_DATA_LEN {
        return None;
    }
    let mut out = [0u8; SEV_SNP_REPORT_DATA_LEN];
    out.copy_from_slice(
        &report[SEV_SNP_REPORT_DATA_OFFSET..SEV_SNP_REPORT_DATA_OFFSET + SEV_SNP_REPORT_DATA_LEN],
    );
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_report_data_truncates_short_input() {
        let short = vec![0u8; 100];
        assert!(extract_report_data(&short).is_none());
    }

    #[test]
    fn extract_report_data_round_trips() {
        let mut report = vec![0u8; SEV_SNP_REPORT_LEN];
        let expected = [7u8; SEV_SNP_REPORT_DATA_LEN];
        report[SEV_SNP_REPORT_DATA_OFFSET..SEV_SNP_REPORT_DATA_OFFSET + SEV_SNP_REPORT_DATA_LEN]
            .copy_from_slice(&expected);
        assert_eq!(extract_report_data(&report), Some(expected));
    }

    #[test]
    fn fetch_report_returns_none_when_device_missing() {
        // /dev/sev-guest doesn't exist on macOS / Linux non-SNP
        // laptops / CI. The fetch should be a clean Ok(None), not an
        // error. On a real SNP guest, accept a well-formed report so
        // the same test suite can run on the VPSBG proof host.
        let report_data = [0u8; 64];
        let result = fetch_report(report_data);
        match result {
            Ok(None) => {} // expected on this test host
            Ok(Some(report)) => {
                assert_eq!(report.len(), SEV_SNP_REPORT_LEN);
                assert_eq!(extract_report_data(&report), Some(report_data));
            }
            Err(e) => panic!("fetch_report should swallow NotFound, got error: {e}"),
        }
    }
}
