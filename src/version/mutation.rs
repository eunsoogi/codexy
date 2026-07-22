use anyhow::Result;
use serde_json::Value;

use super::{
    MARKETPLACE, PLUGIN_MANIFEST, PUBLISH_CONTRACT, admit, cargo, load_json,
    marketplace_plugin_mut, package_manifests, repo_path, write_json,
};

/// Synchronizes plugin, marketplace, and package versions.
///
/// # Errors
///
/// Returns an error when the requested version is invalid, admission fails,
/// required files cannot be read, JSON is invalid, or writes fail.
pub fn set_version(version: &str) -> Result<String> {
    admit(version)?;
    let manifest_path = repo_path(PLUGIN_MANIFEST)?;
    let market_path = repo_path(MARKETPLACE)?;
    let publish_path = repo_path(PUBLISH_CONTRACT)?;
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
