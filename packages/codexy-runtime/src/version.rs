use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::{display_relative, repo_root};

#[cfg(feature = "runtime-activation")]
pub mod activation;
mod admission;
mod bootstrap;
mod cargo;
mod fields;
mod github_plugin;
mod mutation;
mod runtime_selection;
mod wrappers;

const PLUGIN_NAME: &str = "codexy";
const PLUGIN_MANIFEST: &str = "plugins/codexy/.codex-plugin/plugin.json";
const MARKETPLACE: &str = ".agents/plugins/marketplace.json";
const PUBLISH_CONTRACT: &str = ".agents/plugins/release-publish-contract.json";

pub use admission::{VersionAdvanceAdmission, admit};
pub use mutation::set_version;

pub(super) fn repo_path(relative: &str) -> Result<PathBuf> {
    Ok(repo_root()?.join(relative))
}

fn runtime_package_path(root: &std::path::Path, relative: &str) -> PathBuf {
    root.join("packages/codexy-runtime").join(relative)
}

fn package_manifests() -> Result<Vec<PathBuf>> {
    let path = repo_path("package.json")?;
    Ok(if path.exists() {
        vec![path]
    } else {
        Vec::new()
    })
}

pub(super) fn load_json(path: &PathBuf) -> Result<Value> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    serde_json::from_str(&text)
        .with_context(|| format!("invalid JSON in {}", display_relative(path)))
}

pub(super) fn write_json(path: &PathBuf, data: &Value) -> Result<()> {
    let text = format!("{}\n", serde_json::to_string_pretty(data)?);
    fs::write(path, text).with_context(|| format!("writing {}", display_relative(path)))
}

fn require_semver(version: &str) -> Result<()> {
    let mut parts = version.split('.');
    let valid = (0..3).all(|_| {
        let Some(part) = parts.next() else {
            return false;
        };
        !part.is_empty()
            && part.chars().all(|ch| ch.is_ascii_digit())
            && (part == "0" || !part.starts_with('0'))
    }) && parts.next().is_none();
    if valid {
        Ok(())
    } else {
        bail!("version must be semver-like MAJOR.MINOR.PATCH: {version:?}")
    }
}

pub(super) fn require_matching_version(
    version: &str,
    label: &str,
    expected: &str,
    expected_label: &str,
) -> Result<()> {
    require_semver(version)?;
    if version != expected {
        bail!("version mismatch: {label}={version}, {expected_label}={expected}");
    }
    Ok(())
}

pub(super) fn string_field<'a>(data: &'a Value, field: &str, label: &str) -> Result<&'a str> {
    data.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{label} {field} must be a string"))
}

pub(super) fn marketplace_plugin_mut_named<'a>(
    marketplace: &'a mut Value,
    name: &str,
) -> Result<&'a mut Value> {
    let plugins = marketplace
        .get_mut("plugins")
        .and_then(Value::as_array_mut)
        .context(".agents/plugins/marketplace.json must contain a plugins array")?;
    let matches = plugins
        .iter()
        .enumerate()
        .filter(|(_, plugin)| plugin.get("name").and_then(Value::as_str) == Some(name))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        bail!(
            "expected exactly one marketplace plugin named {name:?}, found {}",
            matches.len()
        );
    }
    plugins
        .get_mut(matches[0])
        .context("marketplace plugin index disappeared")
}

fn marketplace_plugin_mut(marketplace: &mut Value) -> Result<&mut Value> {
    marketplace_plugin_mut_named(marketplace, PLUGIN_NAME)
}

/// Checks plugin, marketplace, and package version parity.
///
/// # Errors
///
/// Returns an error when required files are missing, JSON is invalid, versions
/// are malformed, or version values differ.
pub fn check_versions() -> Result<String> {
    check_versions_for_tag(None)
}

pub fn check_versions_for_tag(tag: Option<&str>) -> Result<String> {
    let manifest_path = repo_path(PLUGIN_MANIFEST)?;
    let market_path = repo_path(MARKETPLACE)?;
    let publish_path = repo_path(PUBLISH_CONTRACT)?;
    let manifest = load_json(&manifest_path)?;
    let mut marketplace = load_json(&market_path)?;
    let publish = load_json(&publish_path)?;
    let manifest_version = string_field(&manifest, "version", "plugin manifest")?;
    require_semver(manifest_version)?;
    let marketplace_version = string_field(
        marketplace_plugin_mut(&mut marketplace)?,
        "version",
        "marketplace plugin entry",
    )?;
    require_matching_version(
        marketplace_version,
        &display_relative(&market_path),
        manifest_version,
        &display_relative(&manifest_path),
    )?;
    github_plugin::check(manifest_version)?;
    let manifest_platforms = fields::string_array(
        &manifest,
        "supportedPlatforms",
        &display_relative(&manifest_path),
    )?;
    let marketplace_platforms = fields::string_array(
        marketplace_plugin_mut(&mut marketplace)?,
        "supportedPlatforms",
        "marketplace plugin entry",
    )?;
    if manifest_platforms != marketplace_platforms {
        bail!(
            "supportedPlatforms mismatch: {}={:?}, {}={:?}",
            display_relative(&manifest_path),
            manifest_platforms,
            display_relative(&market_path),
            marketplace_platforms
        );
    }
    let publish_version = string_field(&publish, "version", &display_relative(&publish_path))?;
    require_matching_version(
        publish_version,
        &display_relative(&publish_path),
        manifest_version,
        &display_relative(&manifest_path),
    )?;
    let archive_platforms = publish
        .get("releaseArchive")
        .and_then(Value::as_object)
        .map(|archive| {
            fields::string_array(
                &Value::Object(archive.clone()),
                "platforms",
                "release archive",
            )
        })
        .transpose()?
        .with_context(|| {
            format!(
                "{} releaseArchive must be an object",
                display_relative(&publish_path)
            )
        })?;
    let package_platforms = publish
        .get("package")
        .and_then(Value::as_object)
        .map(|package| {
            fields::string_array(
                &Value::Object(package.clone()),
                "platforms",
                "publish package",
            )
        })
        .transpose()?
        .context("publish package must be an object")?;
    if package_platforms != archive_platforms {
        bail!(
            "release archive platforms mismatch: {} releaseArchive.platforms={:?}, package.platforms={:?}",
            display_relative(&publish_path),
            archive_platforms,
            package_platforms
        );
    }
    for path in package_manifests()? {
        let package = load_json(&path)?;
        let package_version = string_field(&package, "version", &display_relative(&path))?;
        require_matching_version(
            package_version,
            &display_relative(&path),
            manifest_version,
            &display_relative(&manifest_path),
        )?;
    }
    wrappers::check_version(&runtime_selection::wrapper_version(&repo_root()?)?)?;
    cargo::check_version(&repo_root()?, manifest_version)?;
    if let Some(tag) = tag {
        let expected_tag = format!("v{manifest_version}");
        if tag != expected_tag {
            bail!("release tag must be {expected_tag:?}, got {tag:?}");
        }
    }
    Ok(format!(
        "plugin version sync ok: codexy={manifest_version}, codexy-github={manifest_version}"
    ))
}
