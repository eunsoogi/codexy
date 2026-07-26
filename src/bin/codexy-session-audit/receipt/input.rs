use std::{fs, io::Read as _, path::Path};

use anyhow::{Context as _, Result, bail};

use super::super::MAX_INPUT_BYTES;

pub(super) fn read(path: &Path) -> Result<Vec<u8>> {
    let input = fs::File::open(path).context("opening receipt input")?;
    let mut bytes = Vec::new();
    input
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .context("reading receipt input")?;
    if bytes.len() > MAX_INPUT_BYTES {
        bail!("receipt input exceeds {MAX_INPUT_BYTES} bytes");
    }
    Ok(bytes)
}
