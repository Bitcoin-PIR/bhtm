//! Block Hash → MuHash Binding library.
//!
//! This library implements the slim MuHash builder:
//! - stripped block/header binding verification
//! - externally supplied Bitcoin Core-compatible coin deltas
//! - MuHash incremental computation
//! - MuHash Merkle Tree (CT-style append-only binary tree)
//! - SEV-SNP guest-side attestation report fetcher (Unix only)
//! - SEV-SNP attestation triple verifier (`verify_quote`)

pub mod job;
pub mod merkle_tree;
pub mod muhash;
pub mod num3072;
#[cfg(unix)]
pub mod sev_snp_guest;
pub mod stream;
pub mod verify_quote;
