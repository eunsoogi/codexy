use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const PYTHON_MANIFEST: &str = "packages/getcodexy/pyproject.toml";

fn manifest_path() -> Result<PathBuf> {
    Ok(repo_root()?.join(PYTHON_MANIFEST))
}

pub(super) fn check_version(manifest_version: &str) -> Result<()> {
    let path = manifest_path()?;
    let package_version = package_version(&path)?;
    if package_version != manifest_version {
        bail!(
            "version mismatch: {}={package_version}, plugin manifest={manifest_version}",
            display_relative(&path)
        );
    }
    Ok(())
}

pub(super) fn set_version(version: &str) -> Result<()> {
    replace_project_version(&manifest_path()?, version)
}

fn package_version(path: &PathBuf) -> Result<String> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let data = text
        .parse::<toml::Table>()
        .with_context(|| format!("invalid TOML in {}", display_relative(path)))?;
    data.get("project")
        .and_then(toml::Value::as_table)
        .and_then(|project| project.get("version"))
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!(
                "{} project.version must be a string",
                display_relative(path)
            )
        })
}

fn replace_project_version(path: &PathBuf, version: &str) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let mut in_project = false;
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_project = trimmed == "[project]";
        }
        if in_project && trimmed.starts_with("version = ") {
            lines.push(format!("version = \"{version}\""));
            replaced = true;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        bail!("{} project.version line not found", display_relative(path));
    }
    fs::write(path, format!("{}\n", lines.join("\n")))
        .with_context(|| format!("writing {}", display_relative(path)))
}
