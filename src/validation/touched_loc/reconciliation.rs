use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, Result, bail};

const MAIN_REF: &str = "origin/main";

pub(super) struct IntegrationScope {
    head: String,
    reconciliations: Vec<Reconciliation>,
}

struct Reconciliation {
    commit: String,
    main_parent: String,
}

impl IntegrationScope {
    pub(super) fn discover(root: &Path, requested_base: &str) -> Result<Self> {
        let head = child_head(root, requested_base)?;
        let reconciliations = reconciliation_merges(root, requested_base, &head)?;
        Ok(Self {
            head,
            reconciliations,
        })
    }

    pub(super) fn head(&self) -> &str {
        &self.head
    }

    pub(super) fn baseline_for(
        &self,
        root: &Path,
        requested_base: &str,
        path: &Path,
        locally_changed: bool,
    ) -> Result<Option<String>> {
        let mut baseline = None;
        let mut child_changed = false;
        for commit in path_history(root, requested_base, &self.head, path)? {
            let reconciliation = self
                .reconciliations
                .iter()
                .find(|reconciliation| reconciliation.commit == commit);
            if let Some(reconciliation) = reconciliation {
                if !path_differs(root, &reconciliation.main_parent, &commit, path)? {
                    baseline = Some(reconciliation);
                    child_changed = false;
                    continue;
                }
            }
            child_changed = true;
        }
        let Some(reconciliation) = baseline else {
            return Ok(Some(requested_base.to_owned()));
        };
        if !child_changed && !locally_changed {
            return Ok(None);
        }
        Ok(Some(reconciliation.main_parent.clone()))
    }
}

fn child_head(root: &Path, requested_base: &str) -> Result<String> {
    let head = resolve_commit(root, "HEAD")?;
    let requested_base = resolve_commit(root, requested_base)?;
    let parents = commit_parents(root, &head)?;
    if parents.first() == Some(&requested_base) {
        if let Some(child) = parents.get(1) {
            if is_ancestor(root, &requested_base, child)? {
                return Ok(child.clone());
            }
        }
    }
    Ok(head)
}

fn reconciliation_merges(
    root: &Path,
    requested_base: &str,
    head: &str,
) -> Result<Vec<Reconciliation>> {
    if requested_base == MAIN_REF
        || !commit_exists(root, MAIN_REF)?
        || !is_ancestor(root, requested_base, head)?
        || !is_ancestor(root, MAIN_REF, head)?
    {
        return Ok(Vec::new());
    }
    let range = format!("{requested_base}..{head}");
    let output = git(root, ["rev-list", "--first-parent", "--merges", &range])?;
    if !output.status.success() {
        bail!(
            "finding stacked reconciliation merges failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let mut reconciliations = Vec::new();
    for commit in String::from_utf8_lossy(&output.stdout).lines() {
        let parents = commit_parents(root, commit)?;
        let Some(main_parent) = parents.get(1) else {
            continue;
        };
        if is_ancestor(root, main_parent, MAIN_REF)? {
            reconciliations.push(Reconciliation {
                commit: commit.to_owned(),
                main_parent: main_parent.clone(),
            });
        }
    }
    reconciliations.reverse();
    Ok(reconciliations)
}

fn path_history(root: &Path, requested_base: &str, head: &str, path: &Path) -> Result<Vec<String>> {
    let range = format!("{requested_base}..{head}");
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
            "resolving per-path reconciliation history failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let mut commits = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    commits.reverse();
    Ok(commits)
}

fn commit_exists(root: &Path, reference: &str) -> Result<bool> {
    let output = git(root, ["rev-parse", "--verify", "--quiet", reference])?;
    Ok(output.status.success())
}

fn resolve_commit(root: &Path, reference: &str) -> Result<String> {
    let output = git(root, ["rev-parse", "--verify", reference])?;
    if !output.status.success() {
        bail!(
            "resolving touched LOC commit {reference} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn commit_parents(root: &Path, commit: &str) -> Result<Vec<String>> {
    let output = git(root, ["show", "-s", "--format=%P", commit])?;
    if !output.status.success() {
        bail!(
            "resolving touched LOC parents failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect())
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
        "checking whether {reference} is an ancestor of {target} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn path_differs(root: &Path, before: &str, after: &str, path: &Path) -> Result<bool> {
    let output = git(
        root,
        [
            "diff",
            "--quiet",
            before,
            after,
            "--",
            &path.to_string_lossy(),
        ],
    )?;
    if output.status.success() {
        return Ok(false);
    }
    if output.status.code() == Some(1) {
        return Ok(true);
    }
    bail!(
        "comparing reconciliation provenance for {} failed: {}",
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
