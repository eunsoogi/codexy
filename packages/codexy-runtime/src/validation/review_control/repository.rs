use std::{collections::BTreeSet, path::Path, process::Command};

use anyhow::{Result, bail};
use sha2::{Digest as _, Sha256};

pub(super) struct Current {
    pub(super) base_oid: String,
    pub(super) head_oid: String,
    pub(super) diff_sha256: String,
    pub(super) changed_files: BTreeSet<String>,
}

impl Current {
    pub(super) fn load(root: &Path, base: &str) -> Result<Self> {
        let base = git(
            root,
            ["rev-parse", "--verify", &format!("{base}^{{commit}}")],
        )?;
        let head = git(root, ["rev-parse", "HEAD"])?;
        git(root, ["merge-base", "--is-ancestor", &base, &head])?;
        let diff = git_bytes(
            root,
            [
                "diff",
                "--no-ext-diff",
                "--binary",
                &format!("{base}..{head}"),
            ],
        )?;
        let names = git(
            root,
            [
                "diff",
                "--name-only",
                "--diff-filter=ACMRD",
                &format!("{base}..{head}"),
            ],
        )?;
        Ok(Self {
            base_oid: base,
            head_oid: head,
            diff_sha256: format!("{:x}", Sha256::digest(diff)),
            changed_files: names.lines().map(str::to_owned).collect(),
        })
    }
}

pub(super) fn blob_digest(root: &Path, head: &str, path: &str) -> Result<String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        bail!("verification evidence path is unsafe");
    }
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &format!("{head}:{path}")])
        .output()?;
    if !output.status.success() {
        bail!("verification evidence path is absent from current head");
    }
    Ok(format!("{:x}", Sha256::digest(output.stdout)))
}

pub(super) fn current_head(root: &Path) -> Result<String> {
    git(root, ["rev-parse", "HEAD"])
}

fn git<const N: usize>(root: &Path, args: [&str; N]) -> Result<String> {
    Ok(String::from_utf8(git_bytes(root, args)?)?.trim().to_owned())
}
fn git_bytes<const N: usize>(root: &Path, args: [&str; N]) -> Result<Vec<u8>> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() {
        bail!(
            "authoritative Git identity check failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}
