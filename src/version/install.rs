use std::fs;

use anyhow::{Context as _, Result, bail};

use crate::paths::repo_root;

const PATH: &str = "install";

pub(super) fn check_version(version: &str) -> Result<()> {
    let text =
        fs::read_to_string(repo_root()?.join(PATH)).context("missing required file: install")?;
    let expected = format!("getcodexy=={version} codexy-update --pre-session");
    if text.contains(&expected) {
        Ok(())
    } else {
        bail!("install must pin {expected}")
    }
}

pub(super) fn set_version(version: &str) -> Result<()> {
    let path = repo_root()?.join(PATH);
    let text = fs::read_to_string(&path).context("missing required file: install")?;
    let updated = regex::Regex::new(r"getcodexy==[0-9]+\.[0-9]+\.[0-9]+")?
        .replace(&text, format!("getcodexy=={version}"));
    fs::write(path, updated.as_bytes()).context("writing install")
}
