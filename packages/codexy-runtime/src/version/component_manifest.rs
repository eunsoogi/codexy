use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use super::{load_json, repo_path, require_matching_version, write_json};

const MANIFEST: &str = "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json";
const SCHEMA: &str = "getcodexy.component-manifest.v1";

pub(super) fn check(expected_version: &str) -> Result<()> {
    let manifest = load_json(&repo_path(MANIFEST)?)?;
    validate_manifest(&manifest, Some(expected_version))
}

pub(super) fn validate_inputs() -> Result<()> {
    let manifest = load_json(&repo_path(MANIFEST)?)?;
    validate_manifest(&manifest, None)
}

fn validate_manifest(manifest: &Value, expected_version: Option<&str>) -> Result<()> {
    if manifest.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        bail!("component manifest schema must be {SCHEMA:?}");
    }
    versions(manifest, "components", "version", expected_version)?;
    versions(
        manifest,
        "compatibleCombinations",
        "version",
        expected_version,
    )
}

pub(super) fn set_version(version: &str) -> Result<()> {
    let path = repo_path(MANIFEST)?;
    let mut manifest = load_json(&path)?;
    for field in ["components", "compatibleCombinations"] {
        let entries = manifest
            .get_mut(field)
            .and_then(Value::as_array_mut)
            .with_context(|| format!("component manifest {field} must be an array"))?;
        for entry in entries {
            entry["version"] = Value::String(version.to_owned());
        }
    }
    write_json(&path, &manifest)
}

fn versions(manifest: &Value, field: &str, key: &str, expected: Option<&str>) -> Result<()> {
    let entries = manifest
        .get(field)
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty())
        .with_context(|| format!("component manifest {field} must be a non-empty array"))?;
    for entry in entries {
        let version = entry.get(key).and_then(Value::as_str).unwrap_or_default();
        super::require_semver(version)?;
        if let Some(expected) = expected {
            require_matching_version(
                version,
                &format!("component manifest {field}"),
                expected,
                "core plugin manifest",
            )?;
        }
    }
    Ok(())
}
