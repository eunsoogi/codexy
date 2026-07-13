use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

const MAIN_REF: &str = "origin/main";

pub(super) fn exclude_reconciled_main_paths(
    root: &Path,
    requested_base: &str,
    paths: Vec<PathBuf>,
) -> Result<Vec<PathBuf>> {
    if requested_base == MAIN_REF
        || !commit_exists(root, MAIN_REF)?
        || !is_ancestor(root, requested_base)?
        || !is_ancestor(root, MAIN_REF)?
    {
        return Ok(paths);
    }
    let mut scoped = Vec::new();
    for path in paths {
        if !matches_main_tree(root, &path)? {
            scoped.push(path);
        }
    }
    Ok(scoped)
}

fn commit_exists(root: &Path, reference: &str) -> Result<bool> {
    let output = git(root, ["rev-parse", "--verify", "--quiet", reference])?;
    Ok(output.status.success())
}

fn is_ancestor(root: &Path, reference: &str) -> Result<bool> {
    let output = git(root, ["merge-base", "--is-ancestor", reference, "HEAD"])?;
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
