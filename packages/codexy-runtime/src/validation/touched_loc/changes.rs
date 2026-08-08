use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

use super::reconciliation::IntegrationScope;

pub(super) struct ChangedFile {
    pub(super) path: PathBuf,
    pub(super) baseline: String,
}

pub(super) fn scoped(root: &Path, base_ref: &str) -> Result<Vec<ChangedFile>> {
    let integration = IntegrationScope::discover(root, base_ref)?;
    let mut paths = git_diff(root, &format!("{base_ref}...{}", integration.head()))?;
    let mut local = BTreeSet::new();
    local.extend(git_diff(root, "--cached")?);
    local.extend(git_diff(root, "")?);
    local.extend(untracked_files(root)?);
    paths.extend(local.iter().cloned());
    paths.sort();
    paths.dedup();

    let mut files = Vec::new();
    for path in paths {
        let Some(baseline) =
            integration.baseline_for(root, base_ref, &path, local.contains(&path))?
        else {
            continue;
        };
        files.push(ChangedFile { path, baseline });
    }
    Ok(files)
}

fn git_diff(root: &Path, range: &str) -> Result<Vec<PathBuf>> {
    let mut command = Command::new("git");
    command.args(["diff", "--name-only", "--diff-filter=ACMRT"]);
    if !range.is_empty() {
        command.arg(range);
    }
    let output = command
        .current_dir(root)
        .output()
        .context("running git diff for touched LOC validation")?;
    if !output.status.success() {
        bail!(
            "git diff for touched LOC validation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect())
}

fn untracked_files(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(root)
        .output()
        .context("running git ls-files for touched LOC validation")?;
    if !output.status.success() {
        bail!(
            "git ls-files for touched LOC validation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect())
}
