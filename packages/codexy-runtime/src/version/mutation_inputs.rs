use anyhow::Result;

use super::{
    PLUGIN_MANIFEST, PUBLISH_CONTRACT, cargo, component_manifest, devtools_plugin, github_plugin,
    load_json, marketplace_plugin_mut, package_locks, package_manifests, repo_path, require_semver,
    root_package_lock, string_field,
};

/// Validates every mutation-managed input without requiring versions to match.
///
/// Version synchronization deliberately reconciles older coherent metadata, so
/// this boundary rejects malformed inputs before writing while permitting values
/// that the mutation is responsible for bringing into parity.
pub(super) fn validate() -> Result<()> {
    let manifest = load_json(&repo_path(PLUGIN_MANIFEST)?)?;
    let mut marketplace = load_json(&repo_path(super::MARKETPLACE)?)?;
    let publish = load_json(&repo_path(PUBLISH_CONTRACT)?)?;
    for version in [
        string_field(&manifest, "version", "plugin manifest")?,
        string_field(
            marketplace_plugin_mut(&mut marketplace)?,
            "version",
            "marketplace plugin entry",
        )?,
        string_field(&publish, "version", "release publish contract")?,
    ] {
        require_semver(version)?;
    }
    github_plugin::validate_mutation_inputs()?;
    devtools_plugin::validate_mutation_inputs()?;
    component_manifest::validate_inputs()?;
    cargo::validate_inputs(&crate::paths::repo_root()?)?;
    for path in package_manifests()? {
        let package = load_json(&path)?;
        require_semver(string_field(
            &package,
            "version",
            &path.display().to_string(),
        )?)?;
    }
    for path in package_locks()? {
        let lock = load_json(&path)?;
        require_semver(string_field(
            root_package_lock(&lock, &path)?,
            "version",
            &format!("{} root package", path.display()),
        )?)?;
    }
    Ok(())
}
