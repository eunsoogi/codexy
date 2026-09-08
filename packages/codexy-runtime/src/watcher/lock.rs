use std::fs::{File, OpenOptions};
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};

use super::io::reject_link_entry;

#[derive(Debug)]
pub(super) struct LockGuard {
    file: File,
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
        reject_link_entry(path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("opening watcher lock {}", path.display()))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { file })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
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
        let parent = temporary.path().join("not-a-directory");
        File::create(&parent)?;
        let path = parent.join("state.lock");
        let expected = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .expect_err("a file cannot be a lock parent");

        let error = LockGuard::try_acquire(&path).expect_err("lock opening should fail");
        assert!(error.to_string().contains(&expected.to_string()));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_lock_entry() -> Result<()> {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir()?;
        let target = temporary.path().join("target");
        File::create(&target)?;
        let path = temporary.path().join("state.lock");
        symlink(&target, &path)?;
        let error = LockGuard::try_acquire(&path).expect_err("symlinked lock must be rejected");
        assert!(error.to_string().contains("must not be a symlink"));
        Ok(())
    }
}
