use std::collections::BTreeSet;
use std::path::{Component, Path};

use anyhow::{Context as _, Result, bail, ensure};

use super::{ChangeKind, FileChange, FileObservedState, WorkingTreeScope};

#[derive(Debug, Clone)]
pub(super) struct RawChange {
    pub(super) kind: ChangeKind,
    pub(super) previous_path: Option<String>,
    pub(super) current_path: Option<String>,
    pub(super) scopes: BTreeSet<WorkingTreeScope>,
}

impl RawChange {
    pub(super) fn with_observed_state(self, observed_state: FileObservedState) -> FileChange {
        FileChange {
            kind: self.kind,
            previous_path: self.previous_path,
            current_path: self.current_path,
            observed_state,
        }
    }
}

pub(super) fn parse_diff(output: &[u8]) -> Result<Vec<RawChange>> {
    let mut records = output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    let mut changes = Vec::new();
    while let Some(status) = records.next() {
        let status = std::str::from_utf8(status)?;
        let code = status.as_bytes().first().copied().unwrap_or_default();
        let change = if matches!(code, b'R' | b'C') {
            let previous = repository_path(
                records
                    .next()
                    .context("Git rename is missing its old path")?,
            )?;
            let current = repository_path(
                records
                    .next()
                    .context("Git rename is missing its new path")?,
            )?;
            RawChange {
                kind: ChangeKind::Renamed,
                previous_path: Some(previous),
                current_path: Some(current),
                scopes: BTreeSet::new(),
            }
        } else {
            let path = repository_path(records.next().context("Git change is missing its path")?)?;
            RawChange {
                kind: match code {
                    b'A' => ChangeKind::Added,
                    b'D' => ChangeKind::Deleted,
                    b'M' | b'T' => ChangeKind::Modified,
                    _ => bail!("unsupported Git diff status: {status}"),
                },
                previous_path: (code == b'D').then_some(path.clone()),
                current_path: (code != b'D').then_some(path),
                scopes: BTreeSet::new(),
            }
        };
        changes.push(change);
    }
    Ok(changes)
}

pub(super) fn parse_status(output: &[u8], include_untracked: bool) -> Result<Vec<RawChange>> {
    let mut records = output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    let mut changes = Vec::new();
    while let Some(record) = records.next() {
        match record.first().copied() {
            Some(b'?') if include_untracked => {
                let path = repository_path(record.get(2..).context("invalid untracked status")?)?;
                changes.push(RawChange {
                    kind: ChangeKind::Added,
                    previous_path: None,
                    current_path: Some(path),
                    scopes: BTreeSet::from([WorkingTreeScope::Untracked]),
                });
            }
            Some(b'#' | b'?' | b'!') | None => {}
            Some(b'1') => {
                let fields = record.splitn(9, |byte| *byte == b' ').collect::<Vec<_>>();
                ensure!(fields.len() == 9, "invalid Git status record");
                let xy = fields[1];
                let scopes = scopes_for_status(xy)?;
                if scopes.is_empty() {
                    continue;
                }
                let path = repository_path(fields[8])?;
                let deleted = xy[0] == b'D' || xy[1] == b'D';
                changes.push(RawChange {
                    kind: kind_for_status(xy),
                    previous_path: deleted.then_some(path.clone()),
                    current_path: (!deleted).then_some(path),
                    scopes,
                });
            }
            Some(b'2') => {
                let fields = record.splitn(10, |byte| *byte == b' ').collect::<Vec<_>>();
                ensure!(fields.len() == 10, "invalid Git rename status record");
                let scopes = scopes_for_status(fields[1])?;
                let current = repository_path(fields[9])?;
                let previous = repository_path(
                    records
                        .next()
                        .context("Git rename is missing its old path")?,
                )?;
                changes.push(RawChange {
                    kind: ChangeKind::Renamed,
                    previous_path: Some(previous),
                    current_path: Some(current),
                    scopes,
                });
            }
            Some(b'u') => {
                let fields = record.splitn(11, |byte| *byte == b' ').collect::<Vec<_>>();
                ensure!(fields.len() == 11, "invalid Git unmerged status record");
                let scopes = scopes_for_status(fields[1])?;
                let path = repository_path(fields[10])?;
                changes.push(RawChange {
                    kind: ChangeKind::Modified,
                    previous_path: None,
                    current_path: Some(path),
                    scopes,
                });
            }
            Some(code) => bail!("unsupported Git status record: {}", code as char),
        }
    }
    Ok(changes)
}

fn scopes_for_status(xy: &[u8]) -> Result<BTreeSet<WorkingTreeScope>> {
    ensure!(xy.len() == 2, "invalid Git status state");
    let mut scopes = BTreeSet::new();
    if xy[0] != b'.' {
        scopes.insert(WorkingTreeScope::Staged);
    }
    if xy[1] != b'.' {
        scopes.insert(WorkingTreeScope::Unstaged);
    }
    Ok(scopes)
}

const fn kind_for_status(xy: &[u8]) -> ChangeKind {
    if xy[0] == b'D' || xy[1] == b'D' {
        ChangeKind::Deleted
    } else if xy[0] == b'A' || xy[1] == b'A' {
        ChangeKind::Added
    } else {
        ChangeKind::Modified
    }
}

pub(super) fn status_fingerprint(output: &[u8], include_untracked: bool) -> Vec<u8> {
    output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty() && (include_untracked || !record.starts_with(b"? ")))
        .flat_map(|record| record.iter().copied().chain(std::iter::once(0)))
        .collect()
}

fn repository_path(path: &[u8]) -> Result<String> {
    let path = std::str::from_utf8(path)?.to_owned();
    ensure!(!path.is_empty(), "Git returned an empty path");
    let candidate = Path::new(&path);
    ensure!(
        !candidate.is_absolute(),
        "Git returned an absolute path: {path}"
    );
    ensure!(
        candidate.components().all(|component| !matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )),
        "Git returned a path outside the repository: {path}"
    );
    Ok(path)
}

pub(super) fn sorted(mut changes: Vec<FileChange>) -> Vec<FileChange> {
    changes.sort_by(|left, right| {
        let left_path = left
            .current_path
            .as_deref()
            .or(left.previous_path.as_deref())
            .unwrap_or_default();
        let right_path = right
            .current_path
            .as_deref()
            .or(right.previous_path.as_deref())
            .unwrap_or_default();
        left_path
            .cmp(right_path)
            .then_with(|| left.previous_path.cmp(&right.previous_path))
            .then_with(|| left.current_path.cmp(&right.current_path))
    });
    changes
}
