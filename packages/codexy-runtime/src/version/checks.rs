use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::{display_relative, repo_root};

use super::{
    MARKETPLACE, PLUGIN_MANIFEST, PUBLISH_CONTRACT, cargo, component_manifest, devtools_plugin,
    fields, github_plugin, load_json, marketplace_plugin_mut, package_lock, package_manifests,
    repo_path, require_matching_version, require_semver, runtime_selection, string_field, wrappers,
};

pub(super) fn check_versions(tag: Option<&str>, check_runtime_selection: bool) -> Result<String> {
    let manifest_path = repo_path(PLUGIN_MANIFEST)?;
    let manifest = load_json(&manifest_path)?;
    let manifest_version = string_field(&manifest, "version", "plugin manifest")?;
    require_semver(manifest_version)?;
    check_marketplace(&manifest, &manifest_path, manifest_version)?;
    check_plugin_packages(manifest_version)?;
    check_publish_contract(&manifest_path, manifest_version)?;
    check_package_versions(&manifest_path, manifest_version)?;
    check_runtime_versions(manifest_version, check_runtime_selection)?;
    check_tag(tag, manifest_version)?;
    Ok(format!(
        "plugin version sync ok: codexy={manifest_version}, codexy-github={manifest_version}, codexy-devtools={manifest_version}"
    ))
}

fn check_marketplace(manifest: &Value, manifest_path: &Path, manifest_version: &str) -> Result<()> {
    let market_path = repo_path(MARKETPLACE)?;
    let mut marketplace = load_json(&market_path)?;
    let plugin = marketplace_plugin_mut(&mut marketplace)?;
    require_matching_version(
        string_field(plugin, "version", "marketplace plugin entry")?,
        &display_relative(&market_path),
        manifest_version,
        &display_relative(manifest_path),
    )?;
    let manifest_platforms = fields::string_array(
        manifest,
        "supportedPlatforms",
        &display_relative(manifest_path),
    )?;
    let marketplace_platforms =
        fields::string_array(plugin, "supportedPlatforms", "marketplace plugin entry")?;
    if manifest_platforms != marketplace_platforms {
        bail!(
            "supportedPlatforms mismatch: {}={:?}, {}={:?}",
            display_relative(manifest_path),
            manifest_platforms,
            display_relative(&market_path),
            marketplace_platforms
        );
    }
    Ok(())
}

fn check_plugin_packages(manifest_version: &str) -> Result<()> {
    github_plugin::check(manifest_version)?;
    devtools_plugin::check(manifest_version)?;
    component_manifest::check(manifest_version)
}

fn check_publish_contract(manifest_path: &Path, manifest_version: &str) -> Result<()> {
    let publish_path = repo_path(PUBLISH_CONTRACT)?;
    let publish = load_json(&publish_path)?;
    require_matching_version(
        string_field(&publish, "version", &display_relative(&publish_path))?,
        &display_relative(&publish_path),
        manifest_version,
        &display_relative(manifest_path),
    )?;
    let archive_platforms = object_platforms(&publish, "releaseArchive", "release archive")
        .with_context(|| {
            format!(
                "{} releaseArchive must be an object",
                display_relative(&publish_path)
            )
        })?;
    let package_platforms = object_platforms(&publish, "package", "publish package")
        .context("publish package must be an object")?;
    if package_platforms != archive_platforms {
        bail!(
            "release archive platforms mismatch: {} releaseArchive.platforms={:?}, package.platforms={:?}",
            display_relative(&publish_path),
            archive_platforms,
            package_platforms
        );
    }
    Ok(())
}

fn object_platforms(data: &Value, field: &str, label: &str) -> Result<Vec<String>> {
    data.get(field)
        .and_then(Value::as_object)
        .map(|object| fields::string_array(&Value::Object(object.clone()), "platforms", label))
        .transpose()?
        .with_context(|| format!("{label} must be an object"))
}

fn check_package_versions(manifest_path: &Path, manifest_version: &str) -> Result<()> {
    let manifest_label = display_relative(manifest_path);
    for path in package_manifests()? {
        check_package_version(
            &path,
            string_field(&load_json(&path)?, "version", &display_relative(&path))?,
            manifest_version,
            &manifest_label,
        )?;
    }
    for path in package_lock::package_locks()? {
        let lock = load_json(&path)?;
        check_package_version(
            &path,
            string_field(
                package_lock::root_package_lock(&lock, &path)?,
                "version",
                &format!("{} root package", display_relative(&path)),
            )?,
            manifest_version,
            &manifest_label,
        )?;
    }
    Ok(())
}

fn check_package_version(
    path: &Path,
    package_version: &str,
    manifest_version: &str,
    manifest_label: &str,
) -> Result<()> {
    require_matching_version(
        package_version,
        &display_relative(path),
        manifest_version,
        manifest_label,
    )
}

fn check_runtime_versions(manifest_version: &str, check_runtime_selection: bool) -> Result<()> {
    let root = repo_root()?;
    cargo::check_version(&root, manifest_version)?;
    if check_runtime_selection {
        wrappers::check_version(&runtime_selection::wrapper_version(&root)?)?;
    }
    Ok(())
}

fn check_tag(tag: Option<&str>, manifest_version: &str) -> Result<()> {
    if let Some(tag) = tag {
        let expected_tag = format!("v{manifest_version}");
        if tag != expected_tag {
            bail!("release tag must be {expected_tag:?}, got {tag:?}");
        }
    }
    Ok(())
}
