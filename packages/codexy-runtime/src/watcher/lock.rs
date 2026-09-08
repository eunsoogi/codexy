use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};

use super::io::{now_ms, random_hex, reject_link};

const STALE_LOCK_MS: u64 = 120_000;

#[derive(Debug)]
pub(super) struct LockGuard {
    path: PathBuf,
    owner: String,
    file: Option<File>,
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
        let mut file = match open_new_lock(path) {
            Ok(file) => file,
            Err(error)
                if error.kind() == io::ErrorKind::AlreadyExists
                    || is_transient_lock_contention(&error) =>
            {
                if stale(path) {
                    let _ = fs::remove_file(path);
                }
                return Ok(None);
            }
            Err(error) => {
                return Err(anyhow!("creating lock {}: {error}", path.display()));
            }
        };
        file.write_all(owner.as_bytes())
            .context("writing watcher lock")?;
        file.sync_all().context("syncing watcher lock")?;
        Ok(Some(Self {
            path: path.to_path_buf(),
            owner,
            file: Some(file),
        }))
    }
}

fn open_new_lock(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

#[cfg(windows)]
fn is_transient_lock_contention(error: &io::Error) -> bool {
    const ERROR_SHARING_VIOLATION: i32 = 32;
    error.raw_os_error() == Some(ERROR_SHARING_VIOLATION)
}

#[cfg(not(windows))]
const fn is_transient_lock_contention(_error: &io::Error) -> bool {
    false
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        drop(self.file.take());
        let matches = fs::read_to_string(&self.path).is_ok_and(|contents| contents == self.owner);
        if matches {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(windows)]
fn stale(path: &Path) -> bool {
    older_than_stale(path)
}

#[cfg(not(windows))]
fn stale(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return older_than_stale(path);
    };
    let mut fields = contents.split(':');
    let pid = fields.next().and_then(|value| value.parse::<u32>().ok());
    let started = fields.next().and_then(|value| value.parse::<u64>().ok());
    let Some(pid) = pid else {
        return older_than_stale(path);
    };
    if process_alive(pid) {
        return false;
    }
    started.is_some_and(|time| now_ms().saturating_sub(time) >= STALE_LOCK_MS)
        || older_than_stale(path)
}

fn older_than_stale(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age >= Duration::from_millis(STALE_LOCK_MS))
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

#[cfg(all(not(unix), not(windows)))]
fn process_alive(_pid: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_recently_created_empty_lock_for_its_initializer() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("state.lock");
        let initializer = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;

        assert!(LockGuard::try_acquire(&path)?.is_none());
        assert!(
            path.exists(),
            "contender must not reclaim an initializing lock"
        );

        drop(initializer);
        Ok(())
    }

    #[test]
    fn removes_the_lock_after_the_guard_releases_its_file() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("state.lock");
        let guard = LockGuard::try_acquire(&path)?.context("lock was not acquired")?;

        assert!(path.exists());
        drop(guard);
        assert!(!path.exists());
        Ok(())
    }

    #[test]
    fn preserves_the_native_cause_when_lock_creation_fails() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let parent = temporary.path().join("not-a-directory");
        File::create(&parent)?;
        let path = parent.join("state.lock");
        let expected = open_new_lock(&path).expect_err("a file cannot be a lock parent");

        let error = LockGuard::try_acquire(&path).expect_err("lock creation should fail");
        assert!(error.to_string().contains(&expected.to_string()));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn treats_a_real_windows_sharing_violation_as_contention() -> Result<()> {
        use std::os::windows::fs::OpenOptionsExt as _;

        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("state.lock");
        let blocker = OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(0)
            .open(&path)?;
        let native = open_new_lock(&path).expect_err("the held lock must deny sharing");
        assert_eq!(native.raw_os_error(), Some(32));
        assert!(is_transient_lock_contention(&native));
        assert!(LockGuard::try_acquire(&path)?.is_none());
        drop(blocker);
        fs::remove_file(path)?;
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn preserves_a_real_windows_access_denied_as_an_error() -> Result<()> {
        use std::process::Command;

        let temporary = tempfile::tempdir()?;
        let blocked = temporary.path().join("denied");
        fs::create_dir(&blocked)?;
        let icacls = std::env::var_os("SystemRoot")
            .map(|root| PathBuf::from(root).join("System32/icacls.exe"))
            .unwrap_or_else(|| PathBuf::from("icacls"));
        let denied = Command::new(&icacls)
            .arg(&blocked)
            .args(["/inheritance:r", "/deny", "*S-1-1-0:(OI)(CI)(W)"])
            .status()?;
        if !denied.success() {
            bail!("icacls could not deny writes to {}", blocked.display());
        }
        let path = blocked.join("state.lock");
        let native = open_new_lock(&path);
        let actual = LockGuard::try_acquire(&path);
        let reset = Command::new(&icacls)
            .arg(&blocked)
            .args(["/reset", "/T", "/C"])
            .status()?;
        if !reset.success() {
            bail!("icacls could not restore {}", blocked.display());
        }
        let native = native.expect_err("denied creation must fail");
        assert_eq!(native.raw_os_error(), Some(5));
        let actual = actual.expect_err("access denial must not become contention");
        assert!(actual.to_string().contains(&native.to_string()));
        Ok(())
    }
}
