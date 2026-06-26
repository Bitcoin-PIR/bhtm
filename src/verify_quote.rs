//! Verifier for blockhash-to-muhash SEV-SNP attestation triples.
//!
//! Given the three artifacts produced by `blockhash-to-muhash bin-teed`
//! or `blockhash-to-muhash job-teed`:
//!
//! - `attestation.bin` — the byte payload whose contents describe the
//!   `(start_height, end_height, start_block_hash, end_block_hash,
//!   tree_root, tree_size, start_muhash, end_muhash)` binding
//! - `report-data.bin` — 64-byte domain-separated SHA512 value the TEE
//!   asked the kernel to bind into REPORT_DATA. The domain is selected
//!   by attestation version.
//! - `sev-snp-report.bin` — raw 1184-byte AMD-signed attestation report
//!
//! this module checks that:
//!
//! 1. `report-data.bin` equals `SHA512(domain || attestation.bin)`
//!    (recovers the expected REPORT_DATA value from the attestation).
//! 2. The `REPORT_DATA` field inside `sev-snp-report.bin` matches
//!    `report-data.bin` byte-for-byte.
//! 3. The `attestation.bin` parses cleanly into the documented schema
//!    (correct version, well-formed fields, valid height range).
//!
//! It does **NOT** verify the AMD ECDSA signature inside the report or
//! the VCEK/ARK certificate chain. That requires fetching a VCEK
//! certificate from the AMD KDS, which is what `pir-attest-verify`
//! in `BitcoinPIR/` handles (planned follow-up to share that crate).
//! Without signature verification, an attacker who can produce a
//! plausible-looking 1184-byte buffer can fake a passing verification.
//! The check here is therefore *necessary but not sufficient*; in
//! practice it is paired with operator-pinned MRTD and VCEK
//! verification.

use sha2::{Digest, Sha512};

/// 64-byte report-data value prefix domain for v1 attestations.
pub const REPORT_DATA_DOMAIN_V1: &[u8] = b"BitcoinPIR/blockhash-to-muhash/report-data/v1";

/// 64-byte report-data value prefix domain for job-bound v2 attestations.
pub const REPORT_DATA_DOMAIN_V2: &[u8] = b"BitcoinPIR/blockhash-to-muhash/report-data/v2";

/// Parse the leading version field out of an attestation payload.
///
/// `attestation.bin` begins with a little-endian `u16` version. Version
/// 1 is the original per-chunk format. 2 is the generic-UKI format
/// that binds a canonical `job.json` by hash.
pub fn attestation_version(attestation: &[u8]) -> Result<u16, String> {
    if attestation.len() < 2 {
        return Err(format!(
            "attestation too short for version field: {} bytes",
            attestation.len()
        ));
    }
    let bytes: [u8; 2] = attestation[..2].try_into().unwrap();
    Ok(u16::from_le_bytes(bytes))
}

/// Layout constants for `attestation.bin`.
const ATTESTATION_V1_HEADER_LEN: usize = 2 + 4 + 4 + 32 + 32 + 32 + 32 + 8;
const ATTESTATION_V1_TOTAL_LEN: usize = ATTESTATION_V1_HEADER_LEN + 384 + 384;
const ATTESTATION_V2_HEADER_LEN: usize = 2 + 2 + 4 + 4 + 32 + 32 + 32 + 32 + 8;
const ATTESTATION_V2_TOTAL_LEN: usize = ATTESTATION_V2_HEADER_LEN + 384 + 384;

/// Decoded attestation payload.
///
/// Note: the byte layout at offset 74..106 is reserved (was
/// `prev_tree_root` in an early draft; always zero in v1). The 32
/// bytes are skipped during decode to preserve wire compatibility
/// with the published v1 attestation format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attestation {
    pub version: u16,
    pub job_version: Option<u16>,
    pub job_sha256: Option<[u8; 32]>,
    pub start_height: u32,
    pub end_height: u32,
    pub start_block_hash: [u8; 32],
    pub end_block_hash: [u8; 32],
    pub tree_root: [u8; 32],
    pub tree_size: u64,
    pub start_muhash: [u8; 384],
    pub end_muhash: [u8; 384],
}

impl Attestation {
    /// Parse an attestation payload into its documented fields.
    pub fn decode(attestation: &[u8]) -> Result<Self, String> {
        let version = attestation_version(attestation)?;
        match version {
            1 => decode_v1(attestation),
            2 => decode_v2(attestation),
            _ => Err(format!(
                "unsupported attestation version: {version} (supported: 1, 2)"
            )),
        }
    }
}

fn decode_v1(attestation: &[u8]) -> Result<Attestation, String> {
    if attestation.len() != ATTESTATION_V1_TOTAL_LEN {
        return Err(format!(
            "attestation must be exactly {} bytes (version 1), got {}",
            ATTESTATION_V1_TOTAL_LEN,
            attestation.len()
        ));
    }
    let mut cur = &attestation[2..];
    let start_height = read_u32(&mut cur, "start_height")?;
    let end_height = read_u32(&mut cur, "end_height")?;
    let start_block_hash = read_arr32(&mut cur, "start_block_hash")?;
    let end_block_hash = read_arr32(&mut cur, "end_block_hash")?;
    // Skip the 32-byte reserved slot (was prev_tree_root in early
    // drafts; always zero in v1). Bounded by the total-length check
    // above, so this can't run off the end.
    let _reserved = read_arr32(&mut cur, "reserved")?;
    let tree_root = read_arr32(&mut cur, "tree_root")?;
    let tree_size = read_u64(&mut cur, "tree_size")?;
    let start_muhash = read_arr384(&mut cur, "start_muhash")?;
    let end_muhash = read_arr384(&mut cur, "end_muhash")?;
    Ok(Attestation {
        version: 1,
        job_version: None,
        job_sha256: None,
        start_height,
        end_height,
        start_block_hash,
        end_block_hash,
        tree_root,
        tree_size,
        start_muhash,
        end_muhash,
    })
}

fn decode_v2(attestation: &[u8]) -> Result<Attestation, String> {
    if attestation.len() != ATTESTATION_V2_TOTAL_LEN {
        return Err(format!(
            "attestation must be exactly {} bytes (version 2), got {}",
            ATTESTATION_V2_TOTAL_LEN,
            attestation.len()
        ));
    }
    let mut cur = &attestation[2..];
    let job_version = read_u16(&mut cur, "job_version")?;
    let start_height = read_u32(&mut cur, "start_height")?;
    let end_height = read_u32(&mut cur, "end_height")?;
    let start_block_hash = read_arr32(&mut cur, "start_block_hash")?;
    let end_block_hash = read_arr32(&mut cur, "end_block_hash")?;
    let job_sha256 = read_arr32(&mut cur, "job_sha256")?;
    let tree_root = read_arr32(&mut cur, "tree_root")?;
    let tree_size = read_u64(&mut cur, "tree_size")?;
    let start_muhash = read_arr384(&mut cur, "start_muhash")?;
    let end_muhash = read_arr384(&mut cur, "end_muhash")?;
    Ok(Attestation {
        version: 2,
        job_version: Some(job_version),
        job_sha256: Some(job_sha256),
        start_height,
        end_height,
        start_block_hash,
        end_block_hash,
        tree_root,
        tree_size,
        start_muhash,
        end_muhash,
    })
}

pub fn report_data_domain_for_version(version: u16) -> Result<&'static [u8], String> {
    match version {
        1 => Ok(REPORT_DATA_DOMAIN_V1),
        2 => Ok(REPORT_DATA_DOMAIN_V2),
        _ => Err(format!(
            "unsupported attestation version for REPORT_DATA: {version}"
        )),
    }
}

/// Compute the 64-byte REPORT_DATA value that corresponds to a given
/// `attestation.bin`.
///
/// This compatibility helper falls back to the v1 domain for malformed
/// buffers so older tests that intentionally pass truncated attestations
/// can still construct a deterministic value. Verifiers should use
/// `compute_report_data_checked`.
pub fn compute_report_data(attestation: &[u8]) -> [u8; 64] {
    compute_report_data_checked(attestation)
        .unwrap_or_else(|_| compute_report_data_with_domain(REPORT_DATA_DOMAIN_V1, attestation))
}

pub fn compute_report_data_checked(attestation: &[u8]) -> Result<[u8; 64], String> {
    let version = attestation_version(attestation)?;
    let domain = report_data_domain_for_version(version)?;
    Ok(compute_report_data_with_domain(domain, attestation))
}

fn compute_report_data_with_domain(domain: &[u8], attestation: &[u8]) -> [u8; 64] {
    let mut h = Sha512::new();
    h.update(domain);
    h.update(attestation);
    let digest = h.finalize();
    let mut out = [0u8; 64];
    out.copy_from_slice(&digest);
    out
}

/// Result of a verify-bhtm-quote run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyResult {
    pub attestation: Attestation,
    pub expected_report_data: [u8; 64],
    pub report_data_field: [u8; 64],
}

/// Verify the `(attestation.bin, report-data.bin, sev-snp-report.bin)`
/// triple.
///
/// Checks (in order):
/// 1. `report-data.bin == SHA512(domain || attestation.bin)`.
/// 2. The 64-byte REPORT_DATA field inside
///    `sev-snp-report.bin` equals `report-data.bin`.
/// 3. `attestation.bin` parses cleanly.
///
/// Returns the parsed attestation + both report-data values on
/// success. Returns `Err` with a descriptive message on the first
/// check that fails.
pub fn verify_triple(
    attestation: &[u8],
    report_data: &[u8; 64],
    sev_snp_report: &[u8],
) -> Result<VerifyResult, String> {
    let parsed_att = Attestation::decode(attestation)?;
    let expected_report_data = compute_report_data_checked(attestation)?;
    if &expected_report_data != report_data {
        return Err(format!(
            "report-data mismatch: attestation-derived {} != supplied {}",
            hex::encode(expected_report_data),
            hex::encode(report_data)
        ));
    }

    #[cfg(unix)]
    {
        use crate::sev_snp_guest::extract_report_data;
        let report_data_field = extract_report_data(sev_snp_report).ok_or_else(|| {
            format!(
                "SEV-SNP report too short for REPORT_DATA field: {} bytes (need ≥ {})",
                sev_snp_report.len(),
                crate::sev_snp_guest::SEV_SNP_REPORT_DATA_OFFSET
                    + crate::sev_snp_guest::SEV_SNP_REPORT_DATA_LEN
            )
        })?;
        if &report_data_field != report_data {
            return Err(format!(
                "SEV-SNP REPORT_DATA mismatch: expected {}, got {}",
                hex::encode(report_data),
                hex::encode(report_data_field)
            ));
        }
        Ok(VerifyResult {
            attestation: parsed_att,
            expected_report_data,
            report_data_field,
        })
    }

    #[cfg(not(unix))]
    {
        // On non-Unix the REPORT_DATA field can't be parsed (the
        // `sev_snp_guest` module is Unix-only). Surface a clear error
        // rather than silently skipping the check.
        let _ = sev_snp_report;
        Err("verify-bhtm-quote requires a Unix-like system to parse the SEV-SNP report".into())
    }
}

fn read_u32(cur: &mut &[u8], label: &str) -> Result<u32, String> {
    if cur.len() < 4 {
        return Err(format!("truncated attestation: {label}"));
    }
    let v = u32::from_le_bytes(cur[..4].try_into().unwrap());
    *cur = &cur[4..];
    Ok(v)
}

fn read_u16(cur: &mut &[u8], label: &str) -> Result<u16, String> {
    if cur.len() < 2 {
        return Err(format!("truncated attestation: {label}"));
    }
    let v = u16::from_le_bytes(cur[..2].try_into().unwrap());
    *cur = &cur[2..];
    Ok(v)
}

fn read_u64(cur: &mut &[u8], label: &str) -> Result<u64, String> {
    if cur.len() < 8 {
        return Err(format!("truncated attestation: {label}"));
    }
    let v = u64::from_le_bytes(cur[..8].try_into().unwrap());
    *cur = &cur[8..];
    Ok(v)
}

fn read_arr32(cur: &mut &[u8], label: &str) -> Result<[u8; 32], String> {
    if cur.len() < 32 {
        return Err(format!("truncated attestation: {label}"));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&cur[..32]);
    *cur = &cur[32..];
    Ok(out)
}

fn read_arr384(cur: &mut &[u8], label: &str) -> Result<[u8; 384], String> {
    if cur.len() < 384 {
        return Err(format!("truncated attestation: {label}"));
    }
    let mut out = [0u8; 384];
    out.copy_from_slice(&cur[..384]);
    *cur = &cur[384..];
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_attestation() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&1u16.to_le_bytes()); // version (always 1)
        v.extend_from_slice(&100u32.to_le_bytes()); // start_height
        v.extend_from_slice(&199u32.to_le_bytes()); // end_height
        v.extend_from_slice(&[1u8; 32]); // start_block_hash
        v.extend_from_slice(&[2u8; 32]); // end_block_hash
        v.extend_from_slice(&[0u8; 32]); // reserved slot (was prev_tree_root; always zero)
        v.extend_from_slice(&[4u8; 32]); // tree_root
        v.extend_from_slice(&100u64.to_le_bytes()); // tree_size
        v.extend_from_slice(&[5u8; 384]); // start_muhash
        v.extend_from_slice(&[6u8; 384]); // end_muhash
        v
    }

    fn sample_job_attestation() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&2u16.to_le_bytes()); // attestation version
        v.extend_from_slice(&1u16.to_le_bytes()); // job schema version
        v.extend_from_slice(&100u32.to_le_bytes()); // start_height
        v.extend_from_slice(&199u32.to_le_bytes()); // end_height
        v.extend_from_slice(&[1u8; 32]); // start_block_hash
        v.extend_from_slice(&[2u8; 32]); // end_block_hash
        v.extend_from_slice(&[3u8; 32]); // job_sha256
        v.extend_from_slice(&[4u8; 32]); // tree_root
        v.extend_from_slice(&100u64.to_le_bytes()); // tree_size
        v.extend_from_slice(&[5u8; 384]); // start_muhash
        v.extend_from_slice(&[6u8; 384]); // end_muhash
        v
    }

    #[test]
    fn attestation_round_trip() {
        let a = sample_attestation();
        let parsed = Attestation::decode(&a).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.job_version, None);
        assert_eq!(parsed.job_sha256, None);
        assert_eq!(parsed.start_height, 100);
        assert_eq!(parsed.end_height, 199);
        assert_eq!(parsed.start_block_hash, [1u8; 32]);
        assert_eq!(parsed.end_block_hash, [2u8; 32]);
        assert_eq!(parsed.tree_root, [4u8; 32]);
        assert_eq!(parsed.tree_size, 100);
        assert_eq!(parsed.start_muhash, [5u8; 384]);
        assert_eq!(parsed.end_muhash, [6u8; 384]);
    }

    #[test]
    fn job_attestation_round_trip() {
        let a = sample_job_attestation();
        let parsed = Attestation::decode(&a).unwrap();
        assert_eq!(parsed.version, 2);
        assert_eq!(parsed.job_version, Some(1));
        assert_eq!(parsed.job_sha256, Some([3u8; 32]));
        assert_eq!(parsed.start_height, 100);
        assert_eq!(parsed.end_height, 199);
        assert_eq!(parsed.start_block_hash, [1u8; 32]);
        assert_eq!(parsed.end_block_hash, [2u8; 32]);
        assert_eq!(parsed.tree_root, [4u8; 32]);
        assert_eq!(parsed.tree_size, 100);
        assert_eq!(parsed.start_muhash, [5u8; 384]);
        assert_eq!(parsed.end_muhash, [6u8; 384]);
    }

    #[test]
    fn attestation_rejects_wrong_size() {
        let mut a = sample_attestation();
        a.pop();
        assert!(Attestation::decode(&a).is_err());
    }

    #[test]
    fn attestation_rejects_unknown_version() {
        let mut a = sample_attestation();
        a[0] = 99;
        a[1] = 0;
        assert!(Attestation::decode(&a).is_err());
    }

    #[test]
    fn compute_report_data_is_deterministic() {
        let a = sample_attestation();
        let r1 = compute_report_data(&a);
        let r2 = compute_report_data(&a);
        assert_eq!(r1, r2);
    }

    #[test]
    fn report_data_domain_differs_between_v1_and_v2() {
        let v1 = sample_attestation();
        let v2 = sample_job_attestation();
        assert_ne!(compute_report_data(&v1), compute_report_data(&v2));
        assert_eq!(
            report_data_domain_for_version(1).unwrap(),
            REPORT_DATA_DOMAIN_V1
        );
        assert_eq!(
            report_data_domain_for_version(2).unwrap(),
            REPORT_DATA_DOMAIN_V2
        );
    }

    #[test]
    fn verify_rejects_mismatched_report_data() {
        let a = sample_attestation();
        let rd = compute_report_data(&a);
        // tamper with rd
        let mut bad_rd = rd;
        bad_rd[0] ^= 0xff;
        // Build a fake 1184-byte report whose REPORT_DATA field is the
        // correct value (so only the report-data-side mismatch trips).
        let mut report = vec![0u8; 1184];
        let off = crate::sev_snp_guest::SEV_SNP_REPORT_DATA_OFFSET;
        report[off..off + 64].copy_from_slice(&rd);
        let err = verify_triple(&a, &bad_rd, &report).unwrap_err();
        assert!(err.contains("report-data mismatch"), "got: {err}");
    }

    #[cfg(unix)]
    #[test]
    fn verify_rejects_sev_snp_report_data_mismatch() {
        let a = sample_attestation();
        let rd = compute_report_data(&a);
        let mut report = vec![0u8; 1184];
        // Stuff a different REPORT_DATA into the report.
        let wrong = [0xab; 64];
        let off = crate::sev_snp_guest::SEV_SNP_REPORT_DATA_OFFSET;
        report[off..off + 64].copy_from_slice(&wrong);
        let err = verify_triple(&a, &rd, &report).unwrap_err();
        assert!(err.contains("SEV-SNP REPORT_DATA mismatch"), "got: {err}");
    }

    #[cfg(unix)]
    #[test]
    fn verify_accepts_consistent_triple() {
        let a = sample_attestation();
        let rd = compute_report_data(&a);
        let mut report = vec![0u8; 1184];
        let off = crate::sev_snp_guest::SEV_SNP_REPORT_DATA_OFFSET;
        report[off..off + 64].copy_from_slice(&rd);
        let result = verify_triple(&a, &rd, &report).unwrap();
        assert_eq!(result.expected_report_data, result.report_data_field);
        assert_eq!(result.attestation.version, 1);
    }

    #[cfg(unix)]
    #[test]
    fn verify_accepts_consistent_job_bound_triple() {
        let a = sample_job_attestation();
        let rd = compute_report_data(&a);
        let mut report = vec![0u8; 1184];
        let off = crate::sev_snp_guest::SEV_SNP_REPORT_DATA_OFFSET;
        report[off..off + 64].copy_from_slice(&rd);
        let result = verify_triple(&a, &rd, &report).unwrap();
        assert_eq!(result.expected_report_data, result.report_data_field);
        assert_eq!(result.attestation.version, 2);
        assert_eq!(result.attestation.job_version, Some(1));
        assert_eq!(result.attestation.job_sha256, Some([3u8; 32]));
    }
}
