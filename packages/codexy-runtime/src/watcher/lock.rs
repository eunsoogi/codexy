use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};

use super::io::{now_ms, random_hex, reject_link};

const STALE_LOCK_MS: u64 = 120_000;

#[derive(Debug)]
pub(super) struct LockGuard {
    path: PathBuf,
    owner: String,
    _file: File,
}

impl LockGuard {
    pub(super) fn acquire(path: &Path, wait_ms: u64) -> Result<Self> {
        let deadline = std::time::Instant::now() + Duration::from_millis(wait_ms);
        loop {
            if let Some(lock) = Self::try_acquire(path)? {
                return Ok(lock);
            }
            if std::time::Instant::now() >= deadline {
                bail!("watcher state lock is busy: {}", path.display());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub(super) fn try_acquire(path: &Path) -> Result<Option<Self>> {
        if path.exists() {
            reject_link(path)?;
        }
        let owner = format!("{}:{}:{}", std::process::id(), now_ms(), random_hex(8)?);
        let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if stale(path) {
                    let _ = fs::remove_file(path);
                }
                return Ok(None);
            }
            Err(error) => {
                return Err(error).with_context(|| format!("creating lock {}", path.display()));
            }
        };
        file.write_all(owner.as_bytes())
            .context("writing watcher lock")?;
        file.sync_all().context("syncing watcher lock")?;
        Ok(Some(Self {
            path: path.to_path_buf(),
            owner,
            _file: file,
        }))
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let matches = fs::read_to_string(&self.path).is_ok_and(|contents| contents == self.owner);
        if matches {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn stale(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return true;
    };
    let mut fields = contents.split(':');
    let pid = fields.next().and_then(|value| value.parse::<u32>().ok());
    let started = fields.next().and_then(|value| value.parse::<u64>().ok());
    let Some(pid) = pid else { return true };
    if process_alive(pid) {
        return false;
    }
    started.is_none_or(|time| now_ms().saturating_sub(time) >= STALE_LOCK_MS)
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // SAFETY: kill(pid, 0) performs no signal delivery and only probes liveness.
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
fn process_alive(_pid: u32) -> bool {
    false
}
