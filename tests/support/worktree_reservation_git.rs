use std::path::{Path, PathBuf};
use std::process::Command;

use super::{FrozenWorktree, ReservationError, WorktreeSnapshot};

impl FrozenWorktree {
    pub(crate) fn initialize(path: PathBuf) -> Result<Self, ReservationError> {
        let repository = path
            .parent()
            .ok_or_else(|| ReservationError::Harness("frozen path has no parent".into()))?
            .join("repository");
        std::fs::create_dir_all(&repository).map_err(ReservationError::io)?;
        git(&repository, ["init", "--quiet"])?;
        git(
            &repository,
            ["config", "user.email", "reservation@example.test"],
        )?;
        git(&repository, ["config", "user.name", "Reservation Harness"])?;
        std::fs::write(repository.join("frozen.txt"), "frozen\n").map_err(ReservationError::io)?;
        git(&repository, ["add", "frozen.txt"])?;
        git(&repository, ["commit", "--quiet", "-m", "initial"])?;
        git(
            &repository,
            [
                "worktree",
                "add",
                "--quiet",
                "-b",
                "sentinel-review",
                path.to_str()
                    .ok_or_else(|| ReservationError::Harness("non-UTF-8 worktree path".into()))?,
                "HEAD",
            ],
        )?;
        Ok(Self {
            path: canonical(&path),
            repository,
        })
    }

    pub(crate) fn materialize_child(&self, path: &Path) -> Result<(), ReservationError> {
        git(
            &self.repository,
            [
                "worktree",
                "add",
                "--quiet",
                "-b",
                "conflicting-child",
                path.to_str()
                    .ok_or_else(|| ReservationError::Harness("non-UTF-8 child path".into()))?,
                "HEAD",
            ],
        )?;
        Ok(())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn snapshot(&self) -> Result<WorktreeSnapshot, ReservationError> {
        Ok(WorktreeSnapshot {
            path: canonical(&self.path),
            head: git(&self.path, ["rev-parse", "HEAD"])?.trim().to_owned(),
            clean: git(&self.path, ["status", "--porcelain"])?.is_empty(),
        })
    }
}

pub(super) fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn git<const N: usize>(directory: &Path, args: [&str; N]) -> Result<String, ReservationError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .map_err(ReservationError::io)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(ReservationError::Harness(format!(
            "git -C {} failed: {}",
            directory.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
