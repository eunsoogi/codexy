use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::{display_relative, repo_root};

mod cargo;

const PLUGIN_NAME: &str = "codexy";

fn plugin_manifest() -> Result<PathBuf> {
    Ok(repo_root()?
        .join("plugins")
        .join(PLUGIN_NAME)
        .join(".codex-plugin/plugin.json"))
}

fn marketplace_path() -> Result<PathBuf> {
    Ok(repo_root()?.join(".agents/plugins/marketplace.json"))
}

fn release_publish_contract_path() -> Result<PathBuf> {
    Ok(repo_root()?.join(".agents/plugins/release-publish-contract.json"))
}

fn package_manifests() -> Result<Vec<PathBuf>> {
    let path = repo_root()?.join("package.json");
    Ok(if path.exists() {
        vec![path]
    } else {
        Vec::new()
    })
}

fn load_json(path: &PathBuf) -> Result<Value> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    serde_json::from_str(&text)
        .with_context(|| format!("invalid JSON in {}", display_relative(path)))
}

fn write_json(path: &PathBuf, data: &Value) -> Result<()> {
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

fn string_field<'a>(data: &'a Value, field: &str, label: &str) -> Result<&'a str> {
    data.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{label} {field} must be a string"))
}

fn string_array_field(data: &Value, field: &str, label: &str) -> Result<Vec<String>> {
    let values = data
        .get(field)
        .and_then(Value::as_array)
        .with_context(|| format!("{label} {field} must be an array"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|item| !item.trim().is_empty())
                .map(ToOwned::to_owned)
                .with_context(|| format!("{label} {field} must contain only non-empty strings"))
        })
        .collect()
}

fn marketplace_plugin_mut(marketplace: &mut Value) -> Result<&mut Value> {
    let plugins = marketplace
        .get_mut("plugins")
        .and_then(Value::as_array_mut)
        .context(".agents/plugins/marketplace.json must contain a plugins array")?;
    let matches = plugins
        .iter()
        .enumerate()
        .filter(|(_, plugin)| plugin.get("name").and_then(Value::as_str) == Some(PLUGIN_NAME))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        bail!(
            "expected exactly one marketplace plugin named {PLUGIN_NAME:?}, found {}",
            matches.len()
        );
    }
    plugins
        .get_mut(matches[0])
        .context("marketplace plugin index disappeared")
}

/// Checks plugin, marketplace, and package version parity.
///
/// # Errors
///
/// Returns an error when required files are missing, JSON is invalid, versions
/// are malformed, or version values differ.
pub fn check_versions() -> Result<String> {
    let manifest_path = plugin_manifest()?;
    let market_path = marketplace_path()?;
    let publish_path = release_publish_contract_path()?;
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
    require_semver(marketplace_version)?;
    if manifest_version != marketplace_version {
        bail!(
            "version mismatch: {}={manifest_version}, {}={marketplace_version}",
            display_relative(&manifest_path),
            display_relative(&market_path)
        );
    }
    let manifest_platforms = string_array_field(
        &manifest,
        "supportedPlatforms",
        &display_relative(&manifest_path),
    )?;
    let marketplace_platforms = string_array_field(
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
    require_semver(publish_version)?;
    if publish_version != manifest_version {
        bail!(
            "version mismatch: {}={publish_version}, {}={manifest_version}",
            display_relative(&publish_path),
            display_relative(&manifest_path)
        );
    }
    let package_platforms = publish
        .get("package")
        .and_then(Value::as_object)
        .map(|package| {
            string_array_field(
                &Value::Object(package.clone()),
                "platforms",
                "publish package",
            )
        })
        .transpose()?
        .with_context(|| {
            format!(
                "{} package must be an object",
                display_relative(&publish_path)
            )
        })?;
    if package_platforms != manifest_platforms {
        bail!(
            "supportedPlatforms mismatch: {}={:?}, {} package.platforms={:?}",
            display_relative(&manifest_path),
            manifest_platforms,
            display_relative(&publish_path),
            package_platforms
        );
    }
    for path in package_manifests()? {
        let package = load_json(&path)?;
        let package_version = string_field(&package, "version", &display_relative(&path))?;
        require_semver(package_version)?;
        if package_version != manifest_version {
            bail!(
                "version mismatch: {}={package_version}, {}={manifest_version}",
                display_relative(&path),
                display_relative(&manifest_path)
            );
        }
    }
    cargo::check_version(manifest_version)?;
    Ok(format!("plugin version sync ok: {manifest_version}"))
}

/// Synchronizes plugin, marketplace, and package versions.
///
/// # Errors
///
/// Returns an error when the requested version is invalid, required files cannot
/// be read, JSON is invalid, or updated files cannot be written.
pub fn set_version(version: &str) -> Result<String> {
    require_semver(version)?;
    let manifest_path = plugin_manifest()?;
    let market_path = marketplace_path()?;
    let publish_path = release_publish_contract_path()?;
    let mut manifest = load_json(&manifest_path)?;
    let mut marketplace = load_json(&market_path)?;
    let mut publish = load_json(&publish_path)?;
    manifest["version"] = Value::String(version.to_owned());
    marketplace_plugin_mut(&mut marketplace)?["version"] = Value::String(version.to_owned());
    publish["version"] = Value::String(version.to_owned());
    write_json(&manifest_path, &manifest)?;
    write_json(&market_path, &marketplace)?;
    write_json(&publish_path, &publish)?;
    cargo::set_version(version)?;
    for path in package_manifests()? {
        let mut package = load_json(&path)?;
        package["version"] = Value::String(version.to_owned());
        write_json(&path, &package)?;
    }
    Ok(format!("plugin version synchronized to {version}"))
}
