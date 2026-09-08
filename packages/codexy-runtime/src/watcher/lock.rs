use std::fs::{File, OpenOptions};
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};

use super::io::reject_link;

// The file contents are opaque. Upgrades are coordinated: active processes
// must use this OS-lock protocol together, and old sentinel owners must exit
// before the new process starts. Mixed active protocols are unsupported.
#[derive(Debug)]
pub(super) struct LockGuard {
    file: File,
}

impl LockGuard {
    pub(super) fn acquire(path: &Path, wait_ms: u64) -> Result<Self> {
        let file = Self::open(path)?;
        let deadline = std::time::Instant::now() + Duration::from_millis(wait_ms);
        loop {
            if Self::try_lock(&file, path)? {
                return Ok(Self { file });
            }
            if std::time::Instant::now() >= deadline {
                bail!("watcher state lock is busy: {}", path.display());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub(super) fn try_acquire(path: &Path) -> Result<Option<Self>> {
        let file = Self::open(path)?;
        Ok(Self::try_lock(&file, path)?.then_some(Self { file }))
    }

    fn open(path: &Path) -> Result<File> {
        reject_link(path)?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("opening watcher lock {}", path.display()))
    }

    fn try_lock(file: &File, path: &Path) -> Result<bool> {
        match file.try_lock() {
            Ok(()) => Ok(true),
            Err(std::fs::TryLockError::WouldBlock) => Ok(false),
            Err(std::fs::TryLockError::Error(error)) => {
                Err(error).with_context(|| format!("locking watcher lock {}", path.display()))
            }
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn keeps_a_persistent_lock_file_after_guard_releases() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("state.lock");
        let guard = LockGuard::try_acquire(&path)?.context("lock was not acquired")?;

        assert!(path.is_file());
        drop(guard);
        assert!(path.is_file(), "the lock identity must remain stable");
        Ok(())
    }

    #[test]
    fn releases_the_os_lock_after_guard_drops() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("state.lock");
        let guard = LockGuard::try_acquire(&path)?.context("lock was not acquired")?;

        assert!(LockGuard::try_acquire(&path)?.is_none());
        drop(guard);
        let replacement = LockGuard::try_acquire(&path)?.context("lock was not released")?;
        drop(replacement);
        Ok(())
    }

    #[test]
    fn preserves_the_native_cause_when_lock_open_fails() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("missing-parent/state.lock");
        let expected = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .expect_err("a file cannot be a lock parent");

        let error = LockGuard::try_acquire(&path).expect_err("lock opening should fail");
        let native = error
            .downcast_ref::<std::io::Error>()
            .context("lock error lost its native cause")?;
        assert_eq!(native.raw_os_error(), expected.raw_os_error());
        assert!(
            error
                .chain()
                .any(|cause| cause.to_string() == expected.to_string())
        );
        Ok(())
    }

    #[test]
    fn treats_legacy_sentinel_bytes_as_opaque() -> Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("state.lock");
        let legacy = b"1234:1700000000000:deadbeef";
        fs::write(&path, legacy)?;

        let guard = LockGuard::try_acquire(&path)?.context("lock was not acquired")?;
        assert_eq!(fs::read(&path)?, legacy);
        drop(guard);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_lock_entry() -> Result<()> {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir()?;
        let target = temporary.path().join("target");
        fs::write(&target, b"target remains untouched")?;
        let path = temporary.path().join("state.lock");
        symlink(&target, &path)?;
        let error = LockGuard::try_acquire(&path).expect_err("symlinked lock must be rejected");
        assert!(error.to_string().contains("must not be a symlink"));
        assert_eq!(fs::read_link(&path)?, target);
        assert_eq!(fs::read(&target)?, b"target remains untouched");
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
            .map(|root| std::path::PathBuf::from(root).join("System32/icacls.exe"))
            .unwrap_or_else(|| std::path::PathBuf::from("icacls"));
        let denied = Command::new(&icacls)
            .arg(&blocked)
            .args(["/inheritance:r", "/deny", "*S-1-1-0:(OI)(CI)(W)"])
            .status()?;
        if !denied.success() {
            bail!("icacls could not deny writes to {}", blocked.display());
        }
        let path = blocked.join("state.lock");
        let native = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path);
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
