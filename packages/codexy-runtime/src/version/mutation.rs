use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use super::{
    MARKETPLACE, PLUGIN_MANIFEST, PUBLISH_CONTRACT, admit, cargo, component_manifest,
    devtools_plugin, github_plugin, load_json, marketplace_plugin_mut, package_manifests,
    repo_path, runtime_selection, uv_lock, wrappers,
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

pub fn admit_candidate(version: &str) -> Result<String> {
    candidate_preflight(version)?;
    Ok(format!("candidate preparation admission ok: {version}"))
}

pub fn prepare_candidate(version: &str) -> Result<String> {
    let root = crate::paths::repo_root()?;
    candidate_preflight(version)?;
    let selected_runtime_tag = runtime_selection::selected_tag(&root)?;
    let publish_path = repo_path(PUBLISH_CONTRACT)?;
    let mut publish = load_json(&publish_path)?;
    publish["bootstrap"]["candidateVersion"] = Value::String(version.to_owned());
    publish["runtime"]["selectedTag"] = Value::String(selected_runtime_tag);
    let updates = vec![
        uv_lock::prepare_version(version)?,
        uv_lock::prepare_pyproject_version(version)?,
        super::bootstrap::prepare_candidate_version(version)?,
        component_manifest::prepare_version(version)?,
        Update::json(publish_path, &publish)?,
    ];
    for update in updates {
        fs::write(&update.path, update.bytes)?;
    }
    Ok(format!("candidate plugin version prepared for {version}"))
}

pub fn check_candidate() -> Result<String> {
    let root = crate::paths::repo_root()?;
    let selected = super::bootstrap::selected_version()?;
    let candidate = super::bootstrap::candidate_version()?;
    super::require_semver(&selected)?;
    super::require_semver(&candidate)?;
    if semantic(&candidate) <= semantic(&selected) {
        bail!("candidate version {candidate} must advance selected version {selected}");
    }
    let package = uv_lock::package_version()?;
    super::require_matching_version(
        &package,
        "packages/getcodexy/uv.lock",
        &candidate,
        "candidate",
    )?;
    uv_lock::check_pyproject_projection(&candidate)?;

    let manifest = load_json(&repo_path(PLUGIN_MANIFEST)?)?;
    let mut marketplace = load_json(&repo_path(MARKETPLACE)?)?;
    let publish = load_json(&repo_path(PUBLISH_CONTRACT)?)?;
    let manifest_version = super::string_field(&manifest, "version", "plugin manifest")?;
    super::require_matching_version(manifest_version, "plugin manifest", &selected, "selected")?;
    let marketplace_version = super::string_field(
        super::marketplace_plugin_mut(&mut marketplace)?,
        "version",
        "marketplace plugin entry",
    )?;
    super::require_matching_version(
        marketplace_version,
        "marketplace plugin entry",
        &selected,
        "selected",
    )?;
    super::require_matching_version(
        super::string_field(&publish, "version", "release publish contract")?,
        "release publish contract",
        &selected,
        "selected",
    )?;
    let selected_runtime_tag = runtime_selection::selected_tag(&root)?;
    if nested_string(&publish, &["bootstrap", "selectedVersion"])? != selected
        || nested_string(&publish, &["runtime", "selectedTag"])? != selected_runtime_tag
        || nested_string(&publish, &["bootstrap", "candidateVersion"])? != candidate
    {
        bail!("candidate state changed a selected release identity");
    }
    github_plugin::check(&selected)?;
    devtools_plugin::check(&selected)?;
    component_manifest::check(&candidate)?;
    cargo::check_version(&root, &selected)?;
    let prior = runtime_selection::wrapper_version(&root)?;
    wrappers::check_version_at(&root, &prior)?;
    Ok(format!(
        "candidate version state ok: selected={selected}, candidate={candidate}"
    ))
}

fn candidate_preflight(version: &str) -> Result<()> {
    super::require_semver(version)?;
    let root = crate::paths::repo_root()?;
    crate::validation::validate_getcodexy_component_contract(&root.join("plugins/codexy"))?;
    super::mutation_inputs::validate()?;
    super::check_versions_inner(None, true)?;
    super::bootstrap::candidate_version()?;
    let manifest = load_json(&repo_path(PLUGIN_MANIFEST)?)?;
    let selected = super::string_field(&manifest, "version", "plugin manifest")?;
    if semantic(version) <= semantic(selected) {
        bail!("candidate version {version} must advance selected version {selected}");
    }
    Ok(())
}

fn nested_string(value: &Value, fields: &[&str]) -> Result<String> {
    fields
        .iter()
        .try_fold(value, |current, field| {
            current.get(field).context("missing candidate identity")
        })?
        .as_str()
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .context("candidate identity must be a non-empty string")
}

fn semantic(version: &str) -> (u64, u64, u64) {
    let mut parts = version
        .split('.')
        .map(|part| part.parse().unwrap_or(u64::MAX));
    (
        parts.next().unwrap_or(u64::MAX),
        parts.next().unwrap_or(u64::MAX),
        parts.next().unwrap_or(u64::MAX),
    )
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
    publish["currentMarketplace"]["ref"] = Value::String(format!("v{version}"));
    publish["currentMarketplace"]["installCommand"] = Value::String(format!(
        "codex plugin marketplace add eunsoogi/codexy --ref v{version}"
    ));
    let mut updates = vec![
        Update::json(manifest_path, &manifest)?,
        Update::json(publish_path, &publish)?,
    ];
    updates.push(github_plugin::prepare_version(version, &mut marketplace)?);
    if let Some(update) = devtools_plugin::prepare_version(version, &mut marketplace)? {
        updates.push(update);
    }
    updates.push(component_manifest::prepare_version(version)?);
    updates.push(uv_lock::prepare_version(version)?);
    updates.push(uv_lock::prepare_pyproject_version(version)?);
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
