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
mod checks;
mod component_manifest;
mod devtools_plugin;
mod fields;
mod github_plugin;
mod mutation;
mod mutation_inputs;
mod package_lock;
mod runtime_selection;
mod semver;
mod wrappers;

const PLUGIN_NAME: &str = "codexy";
const PLUGIN_MANIFEST: &str = "plugins/codexy/.codex-plugin/plugin.json";
const MARKETPLACE: &str = ".agents/plugins/marketplace.json";
const PUBLISH_CONTRACT: &str = ".agents/plugins/release-publish-contract.json";

pub use admission::{VersionAdvanceAdmission, admit};
pub use mutation::set_version;
pub(crate) use semver::require as require_semver;

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
    checks::check_versions(None, true)
}

pub fn check_versions_for_tag(tag: Option<&str>) -> Result<String> {
    checks::check_versions(tag, true)
}
