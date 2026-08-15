use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

const PACKAGE_NAME: &str = "getcodexy";
const UV_LOCK: &str = "packages/getcodexy/uv.lock";
const PYPROJECT: &str = "packages/getcodexy/pyproject.toml";

fn path() -> Result<PathBuf> {
    super::repo_path(UV_LOCK)
}

fn pyproject_path() -> Result<PathBuf> {
    super::repo_path(PYPROJECT)
}

pub(super) fn package_version() -> Result<String> {
    package_version_from(&path()?)
}

pub(super) fn validate_inputs() -> Result<()> {
    super::require_semver(&package_version()?)
}

pub(super) fn validate_pyproject_projection_input() -> Result<()> {
    super::require_semver(&pyproject_version()?)
}

pub(super) fn check_pyproject_projection(expected: &str) -> Result<()> {
    let pyproject = pyproject_path()?;
    let version = pyproject_version()?;
    super::require_matching_version(&version, &display_relative(&pyproject), expected, UV_LOCK)
}

pub(super) fn prepare_version(version: &str) -> Result<super::mutation::Update> {
    let lock = path()?;
    Ok(super::mutation::Update::bytes(
        lock.clone(),
        replace_package_version(&lock, version)?,
    ))
}

pub(super) fn prepare_pyproject_version(version: &str) -> Result<super::mutation::Update> {
    let pyproject = pyproject_path()?;
    Ok(super::mutation::Update::bytes(
        pyproject.clone(),
        replace_pyproject_version(&pyproject, version)?,
    ))
}

fn package_version_from(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let data = text
        .parse::<toml::Table>()
        .with_context(|| format!("invalid TOML in {}", display_relative(path)))?;
    let packages = data
        .get("package")
        .and_then(toml::Value::as_array)
        .with_context(|| format!("{} must contain package entries", display_relative(path)))?;
    let matches = packages
        .iter()
        .filter_map(toml::Value::as_table)
        .filter(|package| package.get("name").and_then(toml::Value::as_str) == Some(PACKAGE_NAME))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        bail!(
            "expected exactly one {} package named {PACKAGE_NAME:?}, found {}",
            display_relative(path),
            matches.len()
        );
    }
    matches[0]
        .get("version")
        .and_then(toml::Value::as_str)
        .filter(|version| !version.is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!(
                "{} {PACKAGE_NAME} version must be a string",
                display_relative(path)
            )
        })
}

fn pyproject_version() -> Result<String> {
    let pyproject = pyproject_path()?;
    let text = fs::read_to_string(&pyproject)
        .with_context(|| format!("missing required file: {}", display_relative(&pyproject)))?;
    let data = text
        .parse::<toml::Table>()
        .with_context(|| format!("invalid TOML in {}", display_relative(&pyproject)))?;
    data.get("project")
        .and_then(toml::Value::as_table)
        .and_then(|project| project.get("version"))
        .and_then(toml::Value::as_str)
        .filter(|version| !version.is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!(
                "{} project.version must be a string",
                display_relative(&pyproject)
            )
        })
}

fn replace_package_version(path: &Path, version: &str) -> Result<Vec<u8>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let mut matching = false;
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            matching = false;
        } else if trimmed == format!("name = \"{PACKAGE_NAME}\"") {
            matching = true;
        }
        if matching && trimmed.starts_with("version = ") {
            lines.push(format!("version = \"{version}\""));
            replaced = true;
            matching = false;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        bail!(
            "{} package {PACKAGE_NAME:?} version line not found",
            display_relative(path)
        );
    }
    Ok(format!("{}\n", lines.join("\n")).into_bytes())
}

fn replace_pyproject_version(path: &Path, version: &str) -> Result<Vec<u8>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let mut in_project = false;
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "[project]" {
            in_project = true;
        } else if trimmed.starts_with('[') {
            in_project = false;
        }
        if in_project && trimmed.starts_with("version = ") {
            if replaced {
                bail!(
                    "{} project.version appears more than once",
                    display_relative(path)
                );
            }
            lines.push(format!("version = \"{version}\""));
            replaced = true;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        bail!("{} project.version line not found", display_relative(path));
    }
    Ok(format!("{}\n", lines.join("\n")).into_bytes())
}
