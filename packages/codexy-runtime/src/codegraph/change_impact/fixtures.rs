use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use tempfile::TempDir;

use super::super::super::change_input::{ChangeScope, ChangeSet, collect};

pub(super) struct Fixture {
    pub(super) directory: TempDir,
}

impl Fixture {
    pub(super) fn path(&self) -> &Path {
        self.directory.path()
    }

    pub(super) fn write(&self, path: &str, contents: impl AsRef<[u8]>) -> Result<()> {
        let path = self.path().join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
        Ok(())
    }

    pub(super) fn commit(&self, message: &str) -> Result<String> {
        git(self.path(), &["add", "--all"])?;
        git(self.path(), &["commit", "--quiet", "-m", message])?;
        self.head()
    }

    pub(super) fn head(&self) -> Result<String> {
        git(self.path(), &["rev-parse", "HEAD"])
    }

    pub(super) fn comparison(&self, base: &str, head: &str) -> Result<ChangeSet> {
        collect(
            self.path(),
            ChangeScope::head_comparison(base.to_owned(), head.to_owned()),
        )
    }

    pub(super) fn working_tree(&self) -> Result<ChangeSet> {
        collect(self.path(), ChangeScope::working_tree(true))
    }
}

pub(super) fn repository(files: &[(&str, &str)]) -> Result<Fixture> {
    let fixture = Fixture {
        directory: tempfile::tempdir()?,
    };
    git(fixture.path(), &["init", "--quiet"])?;
    git(
        fixture.path(),
        &["config", "user.email", "codexy-tests@example.invalid"],
    )?;
    git(fixture.path(), &["config", "user.name", "Codexy Tests"])?;
    for (path, contents) in files {
        fixture.write(path, contents)?;
    }
    fixture.commit("initial")?;
    Ok(fixture)
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
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
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
