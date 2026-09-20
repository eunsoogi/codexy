mod git;
mod parse;

#[cfg(test)]
mod tests;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeScope {
    HeadComparison { base: String, head: String },
    WorkingTree { include_untracked: bool },
}

impl ChangeScope {
    #[must_use]
    pub fn head_comparison(base: impl Into<String>, head: impl Into<String>) -> Self {
        Self::HeadComparison {
            base: base.into(),
            head: head.into(),
        }
    }

    #[must_use]
    pub const fn working_tree(include_untracked: bool) -> Self {
        Self::WorkingTree { include_untracked }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeSet {
    pub baseline_revision: String,
    pub observed_state: ObservedState,
    pub changes: Vec<FileChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ObservedState {
    HeadComparison {
        requested_head: String,
        head_revision: String,
    },
    WorkingTree {
        scopes: Vec<WorkingTreeScope>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkingTreeScope {
    Staged,
    Unstaged,
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileChange {
    pub kind: ChangeKind,
    pub previous_path: Option<String>,
    pub current_path: Option<String>,
    pub observed_state: FileObservedState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum FileObservedState {
    HeadComparison,
    WorkingTree { scopes: Vec<WorkingTreeScope> },
}

/// Collect a deterministic read-only change set from a Git worktree.
///
/// # Errors
///
/// Returns an error when the root is not the Git worktree root, a revision is
/// invalid, Git reports malformed path data, or the working tree changes while
/// its state is being collected.
pub fn collect(root: &std::path::Path, scope: ChangeScope) -> anyhow::Result<ChangeSet> {
    git::collect(root, scope)
}

/// Collect a change set using the same contract as [`collect`].
///
/// # Errors
///
/// Returns the errors described for [`collect`].
pub fn collect_change_set(root: &std::path::Path, scope: ChangeScope) -> anyhow::Result<ChangeSet> {
    collect(root, scope)
}
