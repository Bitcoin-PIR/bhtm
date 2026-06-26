//! Runtime job descriptors for the generic proof-runner UKI.
//!
//! A generic UKI must not bake chunk-specific URLs or MuHash anchors into
//! its measured command line. Instead it receives a `job.json` at runtime,
//! validates it, canonicalizes it, and binds the canonical job hash into
//! attestation v2.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::Path;

use crate::muhash::MuHash;

pub const JOB_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofJob {
    pub version: u16,
    pub chain: String,
    pub chunk: String,
    pub input_url: String,
    pub input_sha256: String,
    pub upload_url: String,
    pub expected_end_core_muhash: String,
    pub start_anchor: StartAnchor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StartAnchor {
    BeforeGenesis,
    TrustedBlock {
        height: u32,
        block_hash: String,
        muhash: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProofJob {
    pub job: ProofJob,
    pub canonical_json: Vec<u8>,
    pub sha256: [u8; 32],
}

impl ProofJob {
    pub fn load(path: &Path) -> io::Result<LoadedProofJob> {
        let bytes = fs::read(path)?;
        Self::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn from_slice(bytes: &[u8]) -> Result<LoadedProofJob, String> {
        let job: ProofJob =
            serde_json::from_slice(bytes).map_err(|e| format!("invalid job JSON: {e}"))?;
        job.validate()?;
        let canonical_json =
            serde_json::to_vec(&job).map_err(|e| format!("failed to serialize job: {e}"))?;
        let digest = Sha256::digest(&canonical_json);
        let mut sha256 = [0u8; 32];
        sha256.copy_from_slice(&digest);
        Ok(LoadedProofJob {
            job,
            canonical_json,
            sha256,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != JOB_SCHEMA_VERSION {
            return Err(format!(
                "unsupported job version: {} (expected {})",
                self.version, JOB_SCHEMA_VERSION
            ));
        }
        validate_nonempty_no_control(&self.chain, "chain")?;
        validate_nonempty_no_control(&self.chunk, "chunk")?;
        validate_url(&self.input_url, "input_url")?;
        validate_url(&self.upload_url, "upload_url")?;
        validate_hex_len(&self.input_sha256, 32, "input_sha256")?;
        validate_hex_len(
            &self.expected_end_core_muhash,
            32,
            "expected_end_core_muhash",
        )?;
        if self.upload_url.ends_with('/') {
            return Err("upload_url must not end with '/'".into());
        }

        match &self.start_anchor {
            StartAnchor::BeforeGenesis => {}
            StartAnchor::TrustedBlock {
                block_hash, muhash, ..
            } => {
                validate_hex_len(block_hash, 32, "start_anchor.block_hash")?;
                validate_hex_len(muhash, 384, "start_anchor.muhash")?;
            }
        }

        Ok(())
    }

    pub fn expected_start_height(&self) -> u32 {
        match &self.start_anchor {
            StartAnchor::BeforeGenesis => 0,
            StartAnchor::TrustedBlock { height, .. } => *height,
        }
    }

    pub fn expected_start_block_hash_display(&self) -> String {
        match &self.start_anchor {
            StartAnchor::BeforeGenesis => "00".repeat(32),
            StartAnchor::TrustedBlock { block_hash, .. } => block_hash.to_ascii_lowercase(),
        }
    }

    pub fn expected_start_muhash(&self) -> String {
        match &self.start_anchor {
            StartAnchor::BeforeGenesis => hex::encode(MuHash::new().to_bytes()),
            StartAnchor::TrustedBlock { muhash, .. } => muhash.to_ascii_lowercase(),
        }
    }
}

fn validate_nonempty_no_control(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.chars().any(|c| c.is_control()) {
        return Err(format!("{label} must not contain control characters"));
    }
    Ok(())
}

fn validate_url(value: &str, label: &str) -> Result<(), String> {
    validate_nonempty_no_control(value, label)?;
    if !(value.starts_with("https://") || value.starts_with("http://")) {
        return Err(format!("{label} must start with http:// or https://"));
    }
    if value.contains('\'') || value.contains('"') || value.contains('\\') {
        return Err(format!(
            "{label} must not contain quotes or backslashes; use a plain URL"
        ));
    }
    Ok(())
}

fn validate_hex_len(value: &str, expected_bytes: usize, label: &str) -> Result<(), String> {
    let bytes = hex::decode(value).map_err(|e| format!("invalid {label} hex: {e}"))?;
    if bytes.len() != expected_bytes {
        return Err(format!(
            "{label} must be {} bytes / {} hex chars, got {} bytes",
            expected_bytes,
            expected_bytes * 2,
            bytes.len()
        ));
    }
    if value != value.to_ascii_lowercase() {
        return Err(format!("{label} must use lowercase hex"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_job() -> String {
        format!(
            r#"{{
  "upload_url": "https://example.com/chunk/proof",
  "version": 1,
  "expected_end_core_muhash": "{end}",
  "input_sha256": "{input}",
  "chunk": "mainnet-000000-099999",
  "input_url": "https://example.com/chunk/stream.bhtmbin.zst",
  "chain": "mainnet",
  "start_anchor": {{ "kind": "before_genesis" }}
}}"#,
            end = "11".repeat(32),
            input = "22".repeat(32),
        )
    }

    #[test]
    fn job_canonicalization_is_field_order_independent() {
        let loaded = ProofJob::from_slice(sample_job().as_bytes()).unwrap();
        let loaded_again = ProofJob::from_slice(&loaded.canonical_json).unwrap();
        assert_eq!(loaded.sha256, loaded_again.sha256);
        assert_eq!(loaded.canonical_json, loaded_again.canonical_json);
        assert_eq!(loaded.job.expected_start_height(), 0);
    }

    #[test]
    fn trusted_block_anchor_validates_hex_lengths() {
        let job = format!(
            r#"{{
  "version": 1,
  "chain": "mainnet",
  "chunk": "mainnet-100000-199999",
  "input_url": "https://example.com/chunk/stream.bhtmbin.zst",
  "input_sha256": "{input}",
  "upload_url": "https://example.com/chunk/proof",
  "expected_end_core_muhash": "{end}",
  "start_anchor": {{
    "kind": "trusted_block",
    "height": 99999,
    "block_hash": "{block_hash}",
    "muhash": "{muhash}"
  }}
}}"#,
            input = "22".repeat(32),
            end = "33".repeat(32),
            block_hash = "44".repeat(32),
            muhash = "55".repeat(384),
        );
        let loaded = ProofJob::from_slice(job.as_bytes()).unwrap();
        assert_eq!(loaded.job.expected_start_height(), 99999);
        assert_eq!(
            loaded.job.expected_start_block_hash_display(),
            "44".repeat(32)
        );
        assert_eq!(loaded.job.expected_start_muhash(), "55".repeat(384));
    }

    #[test]
    fn job_rejects_unknown_fields() {
        let mut value: serde_json::Value = serde_json::from_str(&sample_job()).unwrap();
        value["extra"] = serde_json::json!(true);
        let err = ProofJob::from_slice(value.to_string().as_bytes()).unwrap_err();
        assert!(err.contains("unknown field"), "got {err}");
    }
}
