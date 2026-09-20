use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail, ensure};

use super::parse::{parse_diff, parse_status, sorted, status_fingerprint};
use super::{ChangeScope, ChangeSet, FileObservedState, ObservedState};

pub(super) fn collect(root: &Path, scope: ChangeScope) -> Result<ChangeSet> {
    let root = repository_root(root)?;
    match scope {
        ChangeScope::HeadComparison { base, head } => collect_comparison(&root, &base, &head),
        ChangeScope::WorkingTree { include_untracked } => {
            collect_working_tree(&root, include_untracked)
        }
    }
}

fn collect_comparison(root: &Path, base: &str, head: &str) -> Result<ChangeSet> {
    let baseline_revision = resolve_revision(root, base)?;
    let head_revision = resolve_revision(root, head)?;
    let output = git_output(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--ignore-submodules=all",
            "--find-renames",
            "--name-status",
            "-z",
            "--diff-filter=ACDMRT",
            &baseline_revision,
            &head_revision,
            "--",
        ],
    )?;
    let changes = parse_diff(&output)?
        .into_iter()
        .map(|change| change.with_observed_state(FileObservedState::HeadComparison))
        .collect::<Vec<_>>();
    Ok(ChangeSet {
        baseline_revision,
        observed_state: ObservedState::HeadComparison {
            requested_head: head.to_owned(),
            head_revision,
        },
        changes: sorted(changes),
    })
}

fn collect_working_tree(root: &Path, include_untracked: bool) -> Result<ChangeSet> {
    collect_working_tree_with_hook(root, include_untracked, || {})
}

pub(super) fn collect_working_tree_with_hook<F>(
    root: &Path,
    include_untracked: bool,
    mut after_first_snapshot: F,
) -> Result<ChangeSet>
where
    F: FnMut(),
{
    let baseline_revision = resolve_revision(root, "HEAD")?;
    let first = status_snapshot(root, include_untracked)?;
    let changes = parse_status(&first.output, include_untracked)?;
    after_first_snapshot();
    let second = status_snapshot(root, include_untracked)?;
    ensure!(
        first.fingerprint == second.fingerprint
            && baseline_revision == resolve_revision(root, "HEAD")?,
        "working tree changed during collection; retry for a stable input"
    );

    let changes = changes
        .into_iter()
        .map(|change| {
            let scopes = change.scopes.iter().copied().collect();
            change.with_observed_state(FileObservedState::WorkingTree { scopes })
        })
        .collect::<Vec<_>>();
    let scopes = changes
        .iter()
        .flat_map(|change| match &change.observed_state {
            FileObservedState::HeadComparison => Vec::new(),
            FileObservedState::WorkingTree { scopes } => scopes.clone(),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(ChangeSet {
        baseline_revision,
        observed_state: ObservedState::WorkingTree { scopes },
        changes: sorted(changes),
    })
}

#[derive(Debug)]
struct StatusSnapshot {
    output: Vec<u8>,
    fingerprint: Vec<u8>,
}

fn repository_root(root: &Path) -> Result<PathBuf> {
    let root = root
        .canonicalize()
        .with_context(|| format!("canonicalizing repository root {}", root.display()))?;
    ensure!(
        root.is_dir(),
        "repository root is not a directory: {}",
        root.display()
    );
    let git_root = String::from_utf8(git_output(&root, &["rev-parse", "--show-toplevel"])?)?;
    let git_root = PathBuf::from(git_root.trim())
        .canonicalize()
        .context("canonicalizing Git repository root")?;
    ensure!(
        root == git_root,
        "repository root must be the Git worktree root: {}",
        root.display()
    );
    Ok(root)
}

fn resolve_revision(root: &Path, revision: &str) -> Result<String> {
    ensure!(!revision.is_empty(), "Git revision must not be empty");
    let requested = format!("{revision}^{{commit}}");
    let output = git_output(
        root,
        &["rev-parse", "--verify", "--end-of-options", &requested],
    )?;
    Ok(String::from_utf8(output)?.trim().to_owned())
}

fn status_snapshot(root: &Path, include_untracked: bool) -> Result<StatusSnapshot> {
    let output = git_output(
        root,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--renames",
            "--ignore-submodules=all",
            "--untracked-files=all",
            "-z",
        ],
    )?;
    let fingerprint = status_fingerprint(&output, include_untracked);
    Ok(StatusSnapshot {
        output,
        fingerprint,
    })
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}
