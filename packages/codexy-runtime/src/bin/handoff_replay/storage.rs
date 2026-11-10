//! Serialize and persist replay records without partial replacements.

use anyhow::{Context as _, Result, ensure};
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

#[cfg(unix)]
use std::os::fd::AsRawFd as _;
#[cfg(windows)]
use std::{os::windows::fs::OpenOptionsExt as _, thread, time::Duration};

pub(super) fn lock_replay(path: &Path) -> Result<File> {
    let name = path.file_name().context("replay path has no file name")?;
    let lock_path = path.with_file_name(format!("{}.lock", name.to_string_lossy()));
    let lock = open_lock(&lock_path)?;
    ensure!(
        fs::symlink_metadata(&lock_path)?.file_type().is_file(),
        "replay lock must be a regular file"
    );
    Ok(lock)
}

fn lock_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    options
}

#[cfg(unix)]
fn open_lock(path: &Path) -> Result<File> {
    let lock = lock_options()
        .open(path)
        .with_context(|| format!("opening replay lock: {}", path.display()))?;
    // SAFETY: flock borrows this live descriptor; the returned File holds the lock.
    let status = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) };
    if status == 0 {
        Ok(lock)
    } else {
        Err(std::io::Error::last_os_error())
            .with_context(|| format!("locking replay: {}", path.display()))
    }
}

#[cfg(windows)]
fn open_lock(path: &Path) -> Result<File> {
    loop {
        match lock_options().share_mode(0).open(path) {
            Ok(file) => return Ok(file),
            Err(error) if matches!(error.raw_os_error(), Some(32 | 33)) => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("opening replay lock: {}", path.display()));
            }
        }
    }
}

pub(super) fn read_replay(path: &Path) -> Result<Vec<String>> {
    match fs::read(path) {
        Ok(bytes) => {
            let value: Value = serde_json::from_slice(&bytes).context("invalid replay JSON")?;
            serde_json::from_value(value).context("replay state must be an envelope array")
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error).with_context(|| format!("reading replay: {}", path.display())),
    }
}

pub(super) fn write_replay(path: &Path, replay: &[String]) -> Result<()> {
    let parent = path.parent().context("replay path has no parent")?;
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(temporary.as_file(), replay)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| anyhow::anyhow!("persisting replay: {}", error.error))?;
    Ok(())
}
