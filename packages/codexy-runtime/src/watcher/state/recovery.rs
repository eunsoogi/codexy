use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use super::super::io::{random_hex, reject_link};

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

pub(super) fn quarantine(root: &Path, path: &Path) -> Result<PathBuf> {
    let parent = root.parent().context("watcher state root has no parent")?;
    let quarantined = parent.join(format!(".codexy-watcher-reclaim-{}", random_hex(16)?));
    fs::rename(path, &quarantined).with_context(|| {
        format!(
            "quarantining expired watcher session directory {}",
            path.display()
        )
    })?;
    Ok(quarantined)
}

pub(super) fn remove_directory(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() {
            reject_link(&entry.path())?;
        }
    }
    fs::remove_dir_all(path).map_err(Into::into)
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
