use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

const MAIN_REF: &str = "origin/main";

pub(super) fn exclude_reconciled_main_paths(
    root: &Path,
    requested_base: &str,
    paths: Vec<PathBuf>,
) -> Result<Vec<PathBuf>> {
    let Some(reconciliation) = reconciliation_merge(root, requested_base)? else {
        return Ok(paths);
    };
    let mut scoped = Vec::new();
    for path in paths {
        if !matches_main_tree(root, &path)? || changed_since(root, &reconciliation, &path)? {
            scoped.push(path);
        }
    }
    Ok(scoped)
}

fn commit_exists(root: &Path, reference: &str) -> Result<bool> {
    let output = git(root, ["rev-parse", "--verify", "--quiet", reference])?;
    Ok(output.status.success())
}

fn reconciliation_merge(root: &Path, requested_base: &str) -> Result<Option<String>> {
    if requested_base == MAIN_REF
        || !commit_exists(root, MAIN_REF)?
        || !is_ancestor(root, requested_base, "HEAD")?
        || !is_ancestor(root, MAIN_REF, "HEAD")?
    {
        return Ok(None);
    }
    let range = format!("{requested_base}..HEAD");
    let output = git(root, ["rev-list", "--first-parent", "--merges", &range])?;
    if !output.status.success() {
        bail!(
            "finding stacked reconciliation merges failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    for commit in String::from_utf8_lossy(&output.stdout).lines() {
        let parents = git(root, ["show", "-s", "--format=%P", commit])?;
        let parent_text = String::from_utf8_lossy(&parents.stdout);
        let Some(main_parent) = parent_text.split_whitespace().nth(1) else {
            continue;
        };
        if is_ancestor(root, main_parent, MAIN_REF)? {
            return Ok(Some(commit.to_owned()));
        }
    }
    Ok(None)
}

fn is_ancestor(root: &Path, reference: &str, target: &str) -> Result<bool> {
    let output = git(root, ["merge-base", "--is-ancestor", reference, target])?;
    if output.status.success() {
        return Ok(true);
    }
    if output.status.code() == Some(1) {
        return Ok(false);
    }
    bail!(
        "checking whether {reference} is an ancestor of HEAD failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn changed_since(root: &Path, reconciliation: &str, path: &Path) -> Result<bool> {
    let range = format!("{reconciliation}..HEAD");
    let output = git(
        root,
        [
            "log",
            "--first-parent",
            "--format=%H",
            &range,
            "--",
            &path.to_string_lossy(),
        ],
    )?;
    if !output.status.success() {
        bail!(
            "checking child changes after reconciliation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(!output.stdout.is_empty())
}

fn matches_main_tree(root: &Path, path: &Path) -> Result<bool> {
    let output = git(
        root,
        [
            "diff",
            "--quiet",
            MAIN_REF,
            "HEAD",
            "--",
            &path.to_string_lossy(),
        ],
    )?;
    if output.status.success() {
        return Ok(true);
    }
    if output.status.code() == Some(1) {
        return Ok(false);
    }
    bail!(
        "comparing {} to {MAIN_REF} failed: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn git<const N: usize>(root: &Path, args: [&str; N]) -> Result<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("resolving touched LOC integration baseline")
}
