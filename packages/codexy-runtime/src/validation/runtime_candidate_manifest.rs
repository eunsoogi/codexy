use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::{Map, Value};

use crate::{
    paths::display_relative,
    validation::runtime_release_schema::{exact, exact_keys, object, object_field, string},
};

const SCHEMA: &str = "codexy-runtime-candidate/v1";

#[cfg(test)]
mod tests;

pub(super) fn check(
    plugin_root: &Path,
    release: &Map<String, Value>,
    release_path: &Path,
) -> Result<()> {
    let path = plugin_root.join("runtime-candidate.json");
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "{} candidate-proven state requires embedded runtime-candidate.json",
            display_relative(release_path)
        )
    })?;
    let outer = string(
        object_field(release, "artifact", release_path)?,
        "sha256",
        release_path,
    )?;
    if text.trim().is_empty() || text.contains(outer) {
        bail!(
            "{} must not self-reference the outer archive SHA",
            display_relative(&path)
        );
    }
    let candidate: Value = serde_json::from_str(&text)
        .with_context(|| format!("invalid JSON in {}", display_relative(&path)))?;
    let candidate = object(&candidate, "candidate receipt", &path)?;
    let core_aware = release.contains_key("classes") || candidate.contains_key("classes");
    let fields = if core_aware {
        &[
            "schema",
            "source",
            "artifact",
            "compatibility",
            "platforms",
            "classes",
        ][..]
    } else {
        &["schema", "source", "artifact", "compatibility", "platforms"][..]
    };
    exact_keys(candidate, fields, &path)?;
    exact(string(candidate, "schema", &path)?, SCHEMA, "schema", &path)?;
    same(candidate, release, "source", &path)?;
    same(candidate, release, "compatibility", &path)?;
    same(candidate, release, "platforms", &path)?;
    if core_aware {
        same(candidate, release, "classes", &path)?;
    }
    let artifact = object_field(candidate, "artifact", &path)?;
    exact_keys(artifact, &["stagingRunId", "stagingRunAttempt"], &path)?;
    for field in ["stagingRunId", "stagingRunAttempt"] {
        positive_staging_identity(artifact, field, &path)?;
    }
    Ok(())
}

fn positive_staging_identity(
    artifact: &Map<String, Value>,
    field: &str,
    path: &Path,
) -> Result<()> {
    if artifact
        .get(field)
        .and_then(Value::as_i64)
        .is_none_or(|value| value < 1)
    {
        bail!(
            "{} {field} must be a positive integer",
            display_relative(path)
        );
    }
    Ok(())
}

fn same(
    candidate: &Map<String, Value>,
    release: &Map<String, Value>,
    field: &str,
    path: &Path,
) -> Result<()> {
    if candidate.get(field) == release.get(field) {
        Ok(())
    } else {
        bail!(
            "{} candidate {field} must match runtime-release.json",
            display_relative(path)
        )
    }
}
