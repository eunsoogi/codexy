use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

const SESSION_TEMP_PREFIX: &str = ".session.json.tmp-";
const SESSION_TEMP_HEX_BYTES: usize = 24;

pub(super) fn reclaim(directory: &Path) -> Result<()> {
    let mut temporary_files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            bail!("watcher session directory is incomplete");
        };
        if !file_type.is_file() || !is_session_temp(name) {
            bail!("watcher session directory is incomplete");
        }
        temporary_files.push(entry.path());
    }
    for path in temporary_files {
        fs::remove_file(path)?;
    }
    fs::remove_dir(directory)?;
    Ok(())
}

fn is_session_temp(name: &str) -> bool {
    name.strip_prefix(SESSION_TEMP_PREFIX)
        .is_some_and(|suffix| {
            suffix.len() == SESSION_TEMP_HEX_BYTES
                && suffix
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
}
