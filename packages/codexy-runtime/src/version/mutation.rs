use std::{fs, path::PathBuf};

use anyhow::Result;
use serde_json::Value;

use super::{
    MARKETPLACE, PLUGIN_MANIFEST, PUBLISH_CONTRACT, admit, cargo, component_manifest,
    devtools_plugin, github_plugin, load_json, marketplace_plugin_mut, package_manifests,
    repo_path,
};

/// A complete managed-file replacement prepared before the mutation commits.
pub(super) struct Update {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl Update {
    pub(super) fn json(path: PathBuf, data: &Value) -> Result<Self> {
        Ok(Self {
            path,
            bytes: format!("{}\n", serde_json::to_string_pretty(data)?).into_bytes(),
        })
    }

    pub(super) fn bytes(path: PathBuf, bytes: Vec<u8>) -> Self {
        Self { path, bytes }
    }
}

/// Synchronizes plugin, marketplace, and package versions.
///
/// # Errors
///
/// Returns an error when the requested version is invalid, admission fails,
/// required files cannot be read, JSON is invalid, or writes fail.
pub fn set_version(version: &str) -> Result<String> {
    let updates = prepare(version)?;
    for update in updates {
        fs::write(&update.path, update.bytes)?;
    }
    Ok(format!("plugin version synchronized to {version}"))
}

fn prepare(version: &str) -> Result<Vec<Update>> {
    let root = preflight(version)?;
    let manifest_path = repo_path(PLUGIN_MANIFEST)?;
    let market_path = repo_path(MARKETPLACE)?;
    let publish_path = repo_path(PUBLISH_CONTRACT)?;
    let mut manifest = load_json(&manifest_path)?;
    let mut marketplace = load_json(&market_path)?;
    let mut publish = load_json(&publish_path)?;
    manifest["version"] = Value::String(version.to_owned());
    marketplace_plugin_mut(&mut marketplace)?["version"] = Value::String(version.to_owned());
    publish["version"] = Value::String(version.to_owned());
    let mut updates = vec![
        Update::json(manifest_path, &manifest)?,
        Update::json(publish_path, &publish)?,
    ];
    updates.push(github_plugin::prepare_version(version, &mut marketplace)?);
    if let Some(update) = devtools_plugin::prepare_version(version, &mut marketplace)? {
        updates.push(update);
    }
    updates.push(component_manifest::prepare_version(version)?);
    updates.extend(cargo::prepare_version(&root, version)?);
    for path in package_manifests()? {
        let mut package = load_json(&path)?;
        package["version"] = Value::String(version.to_owned());
        updates.push(Update::json(path, &package)?);
    }
    updates.push(Update::json(market_path, &marketplace)?);
    Ok(updates)
}

fn preflight(version: &str) -> Result<std::path::PathBuf> {
    let root = crate::paths::repo_root()?;
    crate::validation::validate_getcodexy_component_contract(&root.join("plugins/codexy"))?;
    super::mutation_inputs::validate()?;
    admit(version)?;
    Ok(root)
}
