use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const CARGO_PACKAGE_NAME: &str = "codexy-runtime";

fn cargo_manifest_path() -> Result<PathBuf> {
    Ok(repo_root()?.join("Cargo.toml"))
}

fn cargo_lock_path() -> Result<PathBuf> {
    Ok(repo_root()?.join("Cargo.lock"))
}

pub(super) fn check_version(manifest_version: &str) -> Result<()> {
    let cargo_path = cargo_manifest_path()?;
    let cargo_version = cargo_package_version(&cargo_path)?;
    if cargo_version != manifest_version {
        bail!(
            "version mismatch: {}={cargo_version}, {}={manifest_version}",
            display_relative(&cargo_path),
            "plugin manifest"
        );
    }
    let cargo_lock_path = cargo_lock_path()?;
    let cargo_lock_version = cargo_lock_package_version(&cargo_lock_path)?;
    if cargo_lock_version != manifest_version {
        bail!(
            "version mismatch: {}={cargo_lock_version}, {}={manifest_version}",
            display_relative(&cargo_lock_path),
            "plugin manifest"
        );
    }
    Ok(())
}

pub(super) fn set_version(version: &str) -> Result<()> {
    replace_toml_package_version(&cargo_manifest_path()?, version)?;
    replace_cargo_lock_package_version(&cargo_lock_path()?, version)
}

fn cargo_package_version(path: &PathBuf) -> Result<String> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let data = text
        .parse::<toml::Table>()
        .with_context(|| format!("invalid TOML in {}", display_relative(path)))?;
    let package = data
        .get("package")
        .and_then(toml::Value::as_table)
        .with_context(|| format!("{} must contain a package table", display_relative(path)))?;
    package
        .get("version")
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!(
                "{} package.version must be a string",
                display_relative(path)
            )
        })
}

fn cargo_lock_package_version(path: &PathBuf) -> Result<String> {
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
        .filter(|package| {
            package.get("name").and_then(toml::Value::as_str) == Some(CARGO_PACKAGE_NAME)
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        bail!(
            "expected exactly one Cargo.lock package named {CARGO_PACKAGE_NAME:?}, found {}",
            matches.len()
        );
    }
    matches[0]
        .get("version")
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!(
                "{} {CARGO_PACKAGE_NAME} version must be a string",
                display_relative(path)
            )
        })
}

fn replace_toml_package_version(path: &PathBuf, version: &str) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let mut in_package = false;
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
        }
        if in_package && trimmed.starts_with("version = ") {
            lines.push(format!("version = \"{version}\""));
            replaced = true;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        bail!("{} package.version line not found", display_relative(path));
    }
    fs::write(path, format!("{}\n", lines.join("\n")))
        .with_context(|| format!("writing {}", display_relative(path)))
}

fn replace_cargo_lock_package_version(path: &PathBuf, version: &str) -> Result<()> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let mut in_matching_package = false;
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_matching_package = false;
        } else if trimmed == format!("name = \"{CARGO_PACKAGE_NAME}\"") {
            in_matching_package = true;
        }
        if in_matching_package && trimmed.starts_with("version = ") {
            lines.push(format!("version = \"{version}\""));
            replaced = true;
            in_matching_package = false;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        bail!(
            "{} package {CARGO_PACKAGE_NAME:?} version line not found",
            display_relative(path)
        );
    }
    fs::write(path, format!("{}\n", lines.join("\n")))
        .with_context(|| format!("writing {}", display_relative(path)))
}
