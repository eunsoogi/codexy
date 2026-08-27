use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::{display_relative, repo_root};

#[cfg(feature = "runtime-activation")]
pub mod activation;
mod admission;
mod bootstrap;
mod cargo;
mod component_manifest;
mod devtools_plugin;
mod fields;
mod github_plugin;
mod mutation;
mod mutation_inputs;
mod runtime_selection;
mod semver;
mod uv_lock;
mod wrappers;

const PLUGIN_NAME: &str = "codexy";
const PLUGIN_MANIFEST: &str = "plugins/codexy/.codex-plugin/plugin.json";
const MARKETPLACE: &str = ".agents/plugins/marketplace.json";
const PUBLISH_CONTRACT: &str = ".agents/plugins/release-publish-contract.json";

pub use admission::{VersionAdvanceAdmission, admit};
pub use mutation::{admit_candidate, check_candidate, prepare_candidate, set_version};
pub(crate) use semver::require as require_semver;

#[must_use]
pub const fn runtime_version() -> &'static str {
    bootstrap::CANDIDATE_VERSION
}

pub(super) fn repo_path(relative: &str) -> Result<PathBuf> {
    Ok(repo_root()?.join(relative))
}

fn runtime_package_path(root: &std::path::Path, relative: &str) -> PathBuf {
    root.join("packages/codexy-runtime").join(relative)
}

pub(super) fn package_manifests() -> Result<Vec<PathBuf>> {
    let path = repo_path("package.json")?;
    Ok(if path.exists() {
        vec![path]
    } else {
        Vec::new()
    })
}

pub(super) fn load_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    crate::strict_json::parse(&text)
        .with_context(|| format!("invalid JSON in {}", display_relative(path)))
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

pub(super) fn marketplace_plugin_mut(marketplace: &mut Value) -> Result<&mut Value> {
    marketplace_plugin_mut_named(marketplace, PLUGIN_NAME)
}

/// Checks plugin, marketplace, and package version parity.
///
/// # Errors
///
/// Returns an error when required files are missing, JSON is invalid, versions
/// are malformed, or version values differ.
pub fn check_versions() -> Result<String> {
    check_versions_inner(None, true)
}

pub fn check_versions_for_tag(tag: Option<&str>) -> Result<String> {
    check_versions_inner(tag, true)
}

fn check_versions_inner(tag: Option<&str>, check_runtime_selection: bool) -> Result<String> {
    let manifest_path = repo_path(PLUGIN_MANIFEST)?;
    let market_path = repo_path(MARKETPLACE)?;
    let publish_path = repo_path(PUBLISH_CONTRACT)?;
    let manifest = load_json(&manifest_path)?;
    let mut marketplace = load_json(&market_path)?;
    let publish = load_json(&publish_path)?;
    let manifest_version = string_field(&manifest, "version", "plugin manifest")?;
    require_semver(manifest_version)?;
    let lock_version = uv_lock::package_version()?;
    require_semver(&lock_version)?;
    require_matching_version(
        manifest_version,
        &display_relative(&manifest_path),
        &lock_version,
        "packages/getcodexy/uv.lock",
    )?;
    uv_lock::check_pyproject_projection(&lock_version)?;
    let marketplace_version = string_field(
        marketplace_plugin_mut(&mut marketplace)?,
        "version",
        "marketplace plugin entry",
    )?;
    require_matching_version(
        marketplace_version,
        &display_relative(&market_path),
        &lock_version,
        "packages/getcodexy/uv.lock",
    )?;
    github_plugin::check(&lock_version)?;
    devtools_plugin::check(&lock_version)?;
    component_manifest::check(&lock_version)?;
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
        &lock_version,
        "packages/getcodexy/uv.lock",
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
            &lock_version,
            "packages/getcodexy/uv.lock",
        )?;
    }
    cargo::check_version(&repo_root()?, &lock_version)?;
    if check_runtime_selection {
        wrappers::check_version(&runtime_selection::wrapper_version(&repo_root()?)?)?;
    }
    if let Some(tag) = tag {
        let expected_tag = format!("v{lock_version}");
        if tag != expected_tag {
            bail!("release tag must be {expected_tag:?}, got {tag:?}");
        }
    }
    Ok(format!(
        "plugin version sync ok: codexy={lock_version}, codexy-github={lock_version}, codexy-devtools={lock_version}"
    ))
}
