//! Computes content hashes used by runtime package and activation fixtures.
use sha2::{Digest as _, Sha256};

/// Returns the SHA-256 digest recorded for a fixture artifact.
pub(crate) fn sha256_file(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
