use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[path = "worktree_reservation_git.rs"]
mod worktree_reservation_git;

use worktree_reservation_git::{canonical, snapshot as capture_snapshot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorktreeSnapshot {
    path: PathBuf,
    head: String,
    clean: bool,
}

pub(crate) struct FrozenWorktree {
    pub(super) path: PathBuf,
    pub(super) repository: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskState {
    Active,
    Waiting,
    Terminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReservationRole {
    Sentinel,
}

#[derive(Default)]
pub(crate) struct ReservationRegistry {
    reservations: BTreeMap<String, Reservation>,
    released_paths: BTreeSet<PathBuf>,
}

struct Reservation {
    snapshot: WorktreeSnapshot,
    role: ReservationRole,
    state: TaskState,
    archived: bool,
}

impl ReservationRegistry {
    pub(crate) fn reserve(
        &mut self,
        task_id: &str,
        role: ReservationRole,
        worktree: &FrozenWorktree,
        state: TaskState,
    ) -> Result<(), ReservationError> {
        let snapshot = worktree.snapshot()?;
        if !snapshot.clean {
            return Err(ReservationError::DirtyWorktree(snapshot.path));
        }
        self.released_paths.remove(&snapshot.path);
        self.reservations.insert(
            task_id.to_owned(),
            Reservation {
                snapshot,
                role,
                state,
                archived: false,
            },
        );
        Ok(())
    }

    pub(crate) fn allocate(&self, candidates: &[&Path]) -> Result<PathBuf, ReservationError> {
        let mut collision = None;
        for candidate in candidates {
            let candidate = canonical(candidate);
            if self.is_reserved(&candidate) {
                collision.get_or_insert(candidate);
            } else {
                return Ok(candidate);
            }
        }
        let reserved_path = collision
            .ok_or_else(|| ReservationError::Harness("no allocation candidates".into()))?;
        let expected_snapshot = self.snapshot_for(&reserved_path)?;
        let observed_snapshot = capture_snapshot(&reserved_path)?;
        Err(ReservationError::Collision {
            task_ids: self.task_ids(&reserved_path),
            roles: self.roles(&reserved_path),
            statuses: self.statuses(&reserved_path),
            reserved_path,
            expected_snapshot,
            observed_snapshot,
        })
    }

    pub(crate) fn transition(
        &mut self,
        task_id: &str,
        state: TaskState,
    ) -> Result<(), ReservationError> {
        self.reservation_mut(task_id)?.state = state;
        Ok(())
    }

    pub(crate) fn archive(&mut self, task_id: &str) -> Result<(), ReservationError> {
        let reservation = self.reservation_mut(task_id)?;
        if reservation.state != TaskState::Terminal {
            return Err(ReservationError::ArchiveBeforeTerminal(task_id.to_owned()));
        }
        reservation.archived = true;
        Ok(())
    }

    pub(crate) fn release(&mut self, path: &Path) -> Result<(), ReservationError> {
        let path = canonical(path);
        let task_ids = self.task_ids(&path);
        if task_ids.is_empty() {
            return Err(ReservationError::UnknownWorktree(path));
        }
        if task_ids.iter().any(|task_id| {
            let reservation = &self.reservations[task_id];
            reservation.state != TaskState::Terminal || !reservation.archived
        }) {
            return Err(ReservationError::ReleaseBlocked {
                reserved_path: path,
                task_ids,
            });
        }
        self.released_paths.insert(path);
        Ok(())
    }

    pub(crate) fn is_reserved(&self, path: &Path) -> bool {
        let path = canonical(path);
        !self.released_paths.contains(&path) && !self.task_ids(&path).is_empty()
    }

    fn reservation_mut(&mut self, task_id: &str) -> Result<&mut Reservation, ReservationError> {
        self.reservations
            .get_mut(task_id)
            .ok_or_else(|| ReservationError::UnknownTask(task_id.to_owned()))
    }

    fn task_ids(&self, path: &Path) -> Vec<String> {
        self.reservations
            .iter()
            .filter(|(_, reservation)| reservation.snapshot.path == path)
            .map(|(task_id, _)| task_id.clone())
            .collect()
    }

    fn roles(&self, path: &Path) -> Vec<ReservationRole> {
        self.reservations
            .values()
            .filter(|reservation| reservation.snapshot.path == path)
            .map(|reservation| reservation.role)
            .collect()
    }

    fn statuses(&self, path: &Path) -> Vec<TaskState> {
        self.reservations
            .values()
            .filter(|reservation| reservation.snapshot.path == path)
            .map(|reservation| reservation.state)
            .collect()
    }

    fn snapshot_for(&self, path: &Path) -> Result<WorktreeSnapshot, ReservationError> {
        self.reservations
            .values()
            .find(|reservation| reservation.snapshot.path == path)
            .map(|reservation| reservation.snapshot.clone())
            .ok_or_else(|| ReservationError::UnknownWorktree(path.to_path_buf()))
    }
}

#[derive(Debug)]
pub(crate) enum ReservationError {
    ArchiveBeforeTerminal(String),
    Collision {
        reserved_path: PathBuf,
        roles: Vec<ReservationRole>,
        task_ids: Vec<String>,
        statuses: Vec<TaskState>,
        expected_snapshot: WorktreeSnapshot,
        observed_snapshot: WorktreeSnapshot,
    },
    DirtyWorktree(PathBuf),
    Harness(String),
    ReleaseBlocked {
        reserved_path: PathBuf,
        task_ids: Vec<String>,
    },
    UnknownTask(String),
    UnknownWorktree(PathBuf),
}

impl std::fmt::Display for ReservationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReservationError {}

impl ReservationError {
    fn io(error: std::io::Error) -> Self {
        Self::Harness(error.to_string())
    }
}
