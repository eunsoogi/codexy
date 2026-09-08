use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use super::super::io::{atomic_write, reject_link};
use crate::watcher::lock::LockGuard;

const SESSION_TEMP_PREFIX: &str = ".session.json.tmp-";
const SESSION_TEMP_HEX_BYTES: usize = 24;
const QUARANTINE_PREFIX: &str = ".codexy-watcher-reclaim-";
const MAX_QUARANTINE_SCAN_ENTRIES: usize = 64;
// Fixed slots keep recovery probes bounded even when unrelated parent entries exist.
const MAX_QUARANTINE_SLOTS: usize = super::MAX_SESSIONS;
const RECOVERY_CURSOR_FILE: &str = ".reclaim.cursor";
const MAX_RECOVERY_CURSOR_BYTES: usize = 20;
const OWNED_FILES: [&str; 6] = [
    "state.lock",
    "wait.lock",
    "session.json",
    "health.json",
    "events.jsonl",
    "cancel.json",
];
const ATOMIC_TEMP_PREFIXES: [&str; 3] = [
    ".session.json.tmp-",
    ".health.json.tmp-",
    ".cancel.json.tmp-",
];

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
    for slot in 0..MAX_QUARANTINE_SLOTS {
        let quarantined = quarantine_path(parent, slot);
        match fs::symlink_metadata(&quarantined) {
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        fs::rename(path, &quarantined).with_context(|| {
            format!(
                "quarantining expired watcher session directory {}",
                path.display()
            )
        })?;
        return Ok(quarantined);
    }
    bail!("watcher quarantine storage is full")
}

pub(super) fn recover_quarantines(root: &Path) -> Result<()> {
    let Some(_transition) = LockGuard::try_acquire(&root.join(".reclaim.lock"))? else {
        return Ok(());
    };
    let parent = root.parent().context("watcher state root has no parent")?;
    let cursor = read_cursor(root)? % MAX_QUARANTINE_SLOTS;
    for offset in 0..MAX_QUARANTINE_SCAN_ENTRIES {
        let slot = (cursor + offset) % MAX_QUARANTINE_SLOTS;
        let path = quarantine_path(parent, slot);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_dir() {
            continue;
        }
        recover_quarantine(&path)?;
    }
    write_cursor(
        root,
        (cursor + MAX_QUARANTINE_SCAN_ENTRIES) % MAX_QUARANTINE_SLOTS,
    )?;
    Ok(())
}

fn read_cursor(root: &Path) -> Result<usize> {
    let path = root.join(RECOVERY_CURSOR_FILE);
    reject_link(&path)?;
    let mut file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.by_ref()
        .take((MAX_RECOVERY_CURSOR_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RECOVERY_CURSOR_BYTES {
        return Ok(0);
    }
    Ok(std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .unwrap_or(0))
}

fn write_cursor(root: &Path, cursor: usize) -> Result<()> {
    atomic_write(
        &root.join(RECOVERY_CURSOR_FILE),
        cursor.to_string().as_bytes(),
    )
}

fn quarantine_path(parent: &Path, slot: usize) -> PathBuf {
    parent.join(format!("{QUARANTINE_PREFIX}{slot:032x}"))
}

fn recover_quarantine(path: &Path) -> Result<()> {
    if owned_files_if_known(path)?.is_none() {
        return Ok(());
    }
    let state_path = path.join("state.lock");
    let state_lock = if regular_file(&state_path)? {
        let Some(lock) = LockGuard::try_acquire(&state_path)? else {
            return Ok(());
        };
        Some(lock)
    } else {
        None
    };
    let wait_path = path.join("wait.lock");
    let wait_lock = if regular_file(&wait_path)? {
        let Some(lock) = LockGuard::try_acquire(&wait_path)? else {
            return Ok(());
        };
        Some(lock)
    } else {
        None
    };
    if owned_files_if_known(path)?.is_none() {
        return Ok(());
    }
    drop(wait_lock);
    drop(state_lock);
    remove_directory(path)
}

pub(super) fn remove_directory(path: &Path) -> Result<()> {
    let files = owned_files(path)?;
    for file in files {
        match fs::remove_file(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn owned_files(path: &Path) -> Result<Vec<PathBuf>> {
    let Some(files) = owned_files_if_known(path)? else {
        bail!("watcher quarantine contains an unknown entry");
    };
    Ok(files)
}

fn owned_files_if_known(path: &Path) -> Result<Option<Vec<PathBuf>>> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Some(Vec::new())),
        Err(error) => return Err(error.into()),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Ok(None);
        };
        if !entry.file_type()?.is_file() || !is_owned_file(name) {
            return Ok(None);
        }
        files.push(entry.path());
    }
    Ok(Some(files))
}

fn regular_file(path: &Path) -> Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    Ok(metadata.is_file())
}

fn is_owned_file(name: &str) -> bool {
    OWNED_FILES.contains(&name)
        || ATOMIC_TEMP_PREFIXES.iter().any(|prefix| {
            name.strip_prefix(prefix)
                .is_some_and(|suffix| is_lower_hex(suffix, SESSION_TEMP_HEX_BYTES))
        })
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_session_temp(name: &str) -> bool {
    name.strip_prefix(SESSION_TEMP_PREFIX)
        .is_some_and(|suffix| is_lower_hex(suffix, SESSION_TEMP_HEX_BYTES))
}
