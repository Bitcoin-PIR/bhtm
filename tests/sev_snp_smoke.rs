//! CLI-level smoke tests for the SEV-SNP attestation subcommands.
//!
//! These tests invoke the `blockhash-to-muhash` binary as a subprocess
//! (via `assert_cmd` / `predicates` are not in the dev-dep set, so we
//! use plain `std::process::Command`). The goal is to catch regressions
//! in CLI argument parsing and file I/O wiring; the heavy lifting
//! (REPORT_DATA derivation, attestation parsing, REPORT_DATA field
//! extraction) is already covered by unit tests inside
//! `src/sev_snp_guest.rs` and `src/verify_quote.rs`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use blockhash_to_muhash::job::ProofJob;
use blockhash_to_muhash::muhash::MuHash;
use blockhash_to_muhash::sev_snp_guest::SEV_SNP_REPORT_DATA_OFFSET;
use blockhash_to_muhash::verify_quote::Attestation;
use sha2::{Digest, Sha256};

/// Locate the release-or-debug `blockhash-to-muhash` binary inside
/// `target/`. Cargo guarantees one of the two exists by the time tests
/// run, depending on whether the user invoked `cargo test` or `cargo
/// build --release && cargo test`.
fn binary_path() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR set by cargo for integration tests");
    let target_dir = std::path::Path::new(&manifest_dir).join("target");
    for profile in ["debug", "release"] {
        let candidate = target_dir.join(profile).join("blockhash-to-muhash");
        if candidate.exists() {
            return candidate;
        }
    }
    panic!(
        "no built binary found under target/{{release,debug}}/; run `cargo build` or `cargo build --release` first"
    );
}

fn write_stub_bytes(path: &Path, bytes: &[u8]) {
    let mut f = std::fs::File::create(path).expect("create stub file");
    f.write_all(bytes).expect("write stub file");
}

fn sha256d(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}

fn display_order_hex(hash: &[u8; 32]) -> String {
    hex::encode(hash.iter().rev().copied().collect::<Vec<_>>())
}

fn core_display_muhash(muhash: &[u8; 384]) -> String {
    let digest = Sha256::digest(muhash);
    hex::encode(digest.iter().rev().copied().collect::<Vec<_>>())
}

fn tiny_compact_stream() -> Vec<u8> {
    let mut out = Vec::new();
    let empty_muhash = MuHash::new().to_bytes();
    out.extend_from_slice(b"BHTMBIN1");
    out.push(0); // before-genesis anchor
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 32]);
    out.extend_from_slice(&empty_muhash);

    let stripped_tx = vec![0x01, 0x00, 0x00, 0x00, 0x00];
    let txid = sha256d(&stripped_tx);
    let mut header = [0u8; 80];
    header[..4].copy_from_slice(&1u32.to_le_bytes());
    header[36..68].copy_from_slice(&txid);
    header[68..72].copy_from_slice(&1231006505u32.to_le_bytes());
    header[72..76].copy_from_slice(&0x1d00ffffu32.to_le_bytes());
    header[76..80].copy_from_slice(&1u32.to_le_bytes());
    let block_hash = sha256d(&header);

    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&header);
    out.extend_from_slice(&block_hash);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(stripped_tx.len() as u32).to_le_bytes());
    out.extend_from_slice(&stripped_tx);
    out.extend_from_slice(&0u32.to_le_bytes()); // additions
    out.extend_from_slice(&0u32.to_le_bytes()); // removals
    out
}

#[test]
fn cli_help_lists_new_subcommands() {
    // `main` writes usage to stderr and exits 1 when invoked with no
    // arguments; the usage text is what we want to inspect.
    let output = Command::new(binary_path()).output().expect("spawn binary");
    assert!(!output.status.success(), "no-arg invocation should exit 1");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        stderr.contains("bin-teed"),
        "expected `bin-teed` in usage text, got:\n{stderr}"
    );
    assert!(
        stderr.contains("verify-bhtm-quote"),
        "expected `verify-bhtm-quote` in usage text, got:\n{stderr}"
    );
    assert!(
        stderr.contains("--sev-snp-report-out"),
        "expected `--sev-snp-report-out` flag in usage text"
    );
    assert!(
        !stderr.contains("--prev-tree-root"),
        "`--prev-tree-root` was removed; chunks are independent"
    );
}

#[test]
fn verify_bhtm_quote_rejects_mismatched_sev_snp_report() {
    let dir = tempfile::tempdir().expect("tempdir");
    let attestation_path = dir.path().join("attestation.bin");
    let report_data_path = dir.path().join("report-data.bin");
    let sev_snp_report_path = dir.path().join("sev-snp-report.bin");

    // Use the verify_quote module to build a *consistent* attestation +
    // report-data pair, then construct a SEV-SNP report whose
    // REPORT_DATA field disagrees. The verifier must reject.
    let att = build_consistent_attestation();
    let report_data = blockhash_to_muhash::verify_quote::compute_report_data(&att);

    let mut bad_report = vec![0u8; 1184];
    // Stuff a REPORT_DATA that does NOT match.
    let wrong = [0xab; 64];
    bad_report[SEV_SNP_REPORT_DATA_OFFSET..SEV_SNP_REPORT_DATA_OFFSET + 64].copy_from_slice(&wrong);

    write_stub_bytes(&attestation_path, &att);
    write_stub_bytes(&report_data_path, &report_data);
    write_stub_bytes(&sev_snp_report_path, &bad_report);

    let output = Command::new(binary_path())
        .arg("verify-bhtm-quote")
        .arg(&attestation_path)
        .arg(&report_data_path)
        .arg(&sev_snp_report_path)
        .output()
        .expect("spawn binary");
    assert!(
        !output.status.success(),
        "expected non-zero exit on mismatch"
    );
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        stderr.contains("REPORT_DATA"),
        "expected REPORT_DATA-mismatch error, got: {stderr}"
    );
}

#[test]
fn verify_bhtm_quote_accepts_consistent_triple() {
    let dir = tempfile::tempdir().expect("tempdir");
    let attestation_path = dir.path().join("attestation.bin");
    let report_data_path = dir.path().join("report-data.bin");
    let sev_snp_report_path = dir.path().join("sev-snp-report.bin");

    let att = build_consistent_attestation();
    let report_data = blockhash_to_muhash::verify_quote::compute_report_data(&att);

    let mut good_report = vec![0u8; 1184];
    good_report[SEV_SNP_REPORT_DATA_OFFSET..SEV_SNP_REPORT_DATA_OFFSET + 64]
        .copy_from_slice(&report_data);

    write_stub_bytes(&attestation_path, &att);
    write_stub_bytes(&report_data_path, &report_data);
    write_stub_bytes(&sev_snp_report_path, &good_report);

    let output = Command::new(binary_path())
        .arg("verify-bhtm-quote")
        .arg(&attestation_path)
        .arg(&report_data_path)
        .arg(&sev_snp_report_path)
        .output()
        .expect("spawn binary");
    assert!(
        output.status.success(),
        "expected zero exit on consistent triple; stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(stdout.contains("status=ok"));
    assert!(stdout.contains("report_data_match=ok"));
    assert!(stdout.contains("attestation_version=1"));
}

#[test]
fn job_teed_writes_job_bound_v2_attestation_without_sev_in_warn_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let chunk_path = dir.path().join("chunk.bin");
    let job_path = dir.path().join("job.json");
    let proof_dir = dir.path().join("proof");
    std::fs::create_dir(&proof_dir).expect("create proof dir");

    write_stub_bytes(&chunk_path, &tiny_compact_stream());
    let empty_muhash = MuHash::new().to_bytes();
    let expected_core_muhash = core_display_muhash(&empty_muhash);
    let job = format!(
        r#"{{
  "version": 1,
  "chain": "regtest-smoke",
  "chunk": "tiny-000000",
  "input_url": "https://example.com/tiny/stream.bhtmbin.zst",
  "input_sha256": "{input_sha}",
  "upload_url": "https://example.com/tiny/proof",
  "expected_end_core_muhash": "{expected_core_muhash}",
  "start_anchor": {{ "kind": "before_genesis" }}
}}"#,
        input_sha = "11".repeat(32),
    );
    write_stub_bytes(&job_path, job.as_bytes());
    let loaded_job = ProofJob::from_slice(job.as_bytes()).expect("load job");

    let attestation_path = proof_dir.join("attestation.bin");
    let report_data_path = proof_dir.join("report-data.bin");
    let stats_path = proof_dir.join("stats.json");
    let canonical_job_path = proof_dir.join("job.json");

    let output = Command::new(binary_path())
        .arg("job-teed")
        .arg(&job_path)
        .arg(&chunk_path)
        .arg("--attestation-out")
        .arg(&attestation_path)
        .arg("--report-data-out")
        .arg(&report_data_path)
        .arg("--stats-json-out")
        .arg(&stats_path)
        .arg("--job-out")
        .arg(&canonical_job_path)
        .arg("--sev-snp-report-out")
        .arg(proof_dir.join("sev-snp-report.bin"))
        .arg("--sev-missing")
        .arg("warn")
        .output()
        .expect("spawn binary");
    assert!(
        output.status.success(),
        "expected job-teed success; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let attestation = std::fs::read(&attestation_path).expect("read attestation");
    let parsed = Attestation::decode(&attestation).expect("decode v2 attestation");
    assert_eq!(parsed.version, 2);
    assert_eq!(parsed.job_version, Some(1));
    assert_eq!(parsed.job_sha256, Some(loaded_job.sha256));
    assert_eq!(parsed.start_height, 0);
    assert_eq!(parsed.end_height, 0);
    assert_eq!(display_order_hex(&parsed.start_block_hash), "00".repeat(32));
    assert_eq!(parsed.end_muhash, empty_muhash);

    let report_data = std::fs::read(&report_data_path).expect("read report data");
    assert_eq!(
        report_data.as_slice(),
        &blockhash_to_muhash::verify_quote::compute_report_data(&attestation)
    );
    let canonical_job = std::fs::read(&canonical_job_path).expect("read canonical job");
    assert_eq!(canonical_job, loaded_job.canonical_json);

    let stats: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&stats_path).expect("read stats")).unwrap();
    assert_eq!(stats["attestation_version"], 2);
    assert_eq!(stats["job_sha256"], hex::encode(loaded_job.sha256));
    assert_eq!(stats["end_muhash_core_display"], expected_core_muhash);
}

#[test]
fn verify_bhtm_quote_rejects_truncated_attestation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let attestation_path = dir.path().join("attestation.bin");
    let report_data_path = dir.path().join("report-data.bin");
    let sev_snp_report_path = dir.path().join("sev-snp-report.bin");

    let truncated = vec![0u8; 100];
    let report_data = blockhash_to_muhash::verify_quote::compute_report_data(&truncated);
    let report = vec![0u8; 1184];

    write_stub_bytes(&attestation_path, &truncated);
    write_stub_bytes(&report_data_path, &report_data);
    write_stub_bytes(&sev_snp_report_path, &report);

    let output = Command::new(binary_path())
        .arg("verify-bhtm-quote")
        .arg(&attestation_path)
        .arg(&report_data_path)
        .arg(&sev_snp_report_path)
        .output()
        .expect("spawn binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    // Either the report-data derivation step or the attestation decode
    // step must trip. We don't care which, as long as the binary
    // refuses to print status=ok.
    assert!(
        !stderr.contains("status=ok"),
        "verifier must refuse truncated attestation; stderr: {stderr}"
    );
}

/// Build a structurally-valid version-2 attestation payload with all
/// fields set to arbitrary-but-distinct bytes. Used by the
/// accept/reject tests above.
fn build_consistent_attestation() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&1u16.to_le_bytes()); // version (always 1)
    v.extend_from_slice(&100u32.to_le_bytes()); // start_height
    v.extend_from_slice(&199u32.to_le_bytes()); // end_height
    v.extend_from_slice(&[1u8; 32]); // start_block_hash
    v.extend_from_slice(&[2u8; 32]); // end_block_hash
    v.extend_from_slice(&[0u8; 32]); // reserved (was prev_tree_root; always zero)
    v.extend_from_slice(&[4u8; 32]); // tree_root
    v.extend_from_slice(&100u64.to_le_bytes()); // tree_size
    v.extend_from_slice(&[5u8; 384]); // start_muhash
    v.extend_from_slice(&[6u8; 384]); // end_muhash
    v
}
