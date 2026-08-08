use sha2::{Digest as _, Sha256};

pub(crate) fn sha256_file(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
