use std::path::Path;

use anyhow::{Result, bail};
use serde_json::{Map, Value};

use crate::{
    paths::display_relative,
    validation::{
        load_json,
        runtime_release_schema::{
            digest, exact, exact_keys, integer, object, object_field, string,
        },
    },
};

use super::core;
mod platforms;

const SCHEMA: &str = "codexy-runtime-release/v1";
const REPOSITORY: &str = "https://github.com/eunsoogi/codexy";
const LEGACY_TAG: &str = "v1.2.2";
const LEGACY_PLATFORMS: &[&str] = &["darwin-arm64", "linux-x86_64"];
const PROVENANCE_FIELDS: &[&str] = &[
    "repositoryId",
    "workflowPath",
    "runId",
    "runAttempt",
    "workflowRunUrl",
];
const PROVENANCE_WORKFLOW: &str = ".github/workflows/runtime-candidate.yml";
const REPOSITORY_ID: i64 = 1_269_350_143;
const BASE_FIELDS: &[&str] = &[
    "schema",
    "state",
    "source",
    "artifact",
    "compatibility",
    "platforms",
];
const CANDIDATE_CORE_FIELDS: &[&str] = &[
    "schema",
    "state",
    "source",
    "artifact",
    "compatibility",
    "platforms",
    "classes",
];
const SOURCE_FIELDS: &[&str] = &[
    "schema",
    "state",
    "source",
    "artifact",
    "provenance",
    "compatibility",
    "platforms",
];
const SOURCE_CORE_FIELDS: &[&str] = &[
    "schema",
    "state",
    "source",
    "artifact",
    "provenance",
    "compatibility",
    "platforms",
    "classes",
];
const COMPATIBILITY_FIELDS: &[&str] = &[
    "bootstrapApi",
    "pluginRuntimeApi",
    "transport",
    "mcpProtocol",
];
const SERVERS: [&str; 2] = ["lsp", "codegraph"];

pub(super) fn check(plugin_root: &Path, supported: &[String]) -> Result<()> {
    let path = plugin_root.join("runtime-release.json");
    let document = load_json(&path)?;
    let root = object(&document, "root", &path)?;
    exact(string(root, "schema", &path)?, SCHEMA, "schema", &path)?;
    let state = string(root, "state", &path)?;
    let source = object_field(root, "source", &path)?;
    let core_aware = root.contains_key("classes") || source.contains_key("tree");
    if state == "candidate-proven" && root.contains_key("provenance") {
        core::check_source_surface(plugin_root, supported)?;
    }
    let fields = match (state, core_aware) {
        ("source-selected", true) => SOURCE_CORE_FIELDS,
        ("source-selected", false) => SOURCE_FIELDS,
        ("candidate-proven", true) => CANDIDATE_CORE_FIELDS,
        ("candidate-proven", false) | ("legacy-public", _) => BASE_FIELDS,
        _ => bail!("{} state is unsupported", display_relative(&path)),
    };
    exact_keys(root, fields, &path)?;
    core::check_source(source, state, REPOSITORY, &path)?;
    check_artifact(object_field(root, "artifact", &path)?, state, &path)?;
    check_compatibility(object_field(root, "compatibility", &path)?, &path)?;
    if state == "source-selected" {
        check_provenance(object_field(root, "provenance", &path)?, &path)?;
    }
    platforms::check_platforms(
        object_field(root, "platforms", &path)?,
        supported,
        state,
        &path,
    )?;
    if core_aware && matches!(state, "candidate-proven" | "source-selected") {
        core::check(
            object_field(root, "classes", &path)?,
            source,
            object_field(root, "platforms", &path)?,
            &path,
        )?;
        if state == "candidate-proven" {
            core::check_manifest(plugin_root, root, &path)?;
        }
    }
    if state == "candidate-proven" {
        crate::validation::runtime_candidate_manifest::check(plugin_root, root, &path)?;
    }
    Ok(())
}

fn check_artifact(artifact: &Map<String, Value>, state: &str, path: &Path) -> Result<()> {
    exact_keys(
        artifact,
        &["tag", "url", "sha256", "payloadManifestSha256"],
        path,
    )?;
    let tag = string(artifact, "tag", path)?;
    check_tag(tag, state, path)?;
    let asset = match state {
        "legacy-public" => "codexy-marketplace-plugin.tar.gz",
        "candidate-proven" | "source-selected" => "codexy-runtime-package.tar.gz",
        _ => bail!("{} state is unsupported", display_relative(path)),
    };
    exact(
        string(artifact, "url", path)?,
        &format!("{REPOSITORY}/releases/download/{tag}/{asset}"),
        "artifact.url",
        path,
    )?;
    digest(string(artifact, "sha256", path)?, "artifact.sha256", path)?;
    digest(
        string(artifact, "payloadManifestSha256", path)?,
        "artifact.payloadManifestSha256",
        path,
    )?;
    if state == "legacy-public" {
        exact(tag, LEGACY_TAG, "artifact.tag", path)?;
    }
    Ok(())
}

fn check_tag(tag: &str, state: &str, path: &Path) -> Result<()> {
    if state == "legacy-public" {
        return exact(tag, LEGACY_TAG, "artifact.tag", path);
    }
    let version = tag.strip_prefix('v').unwrap_or_default();
    if version.split('.').count() == 3
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        Ok(())
    } else {
        bail!(
            "{} artifact.tag must be a version-only vMAJOR.MINOR.PATCH tag",
            display_relative(path)
        )
    }
}

fn check_provenance(value: &Map<String, Value>, path: &Path) -> Result<()> {
    exact_keys(value, PROVENANCE_FIELDS, path)?;
    integer(value, "repositoryId", path, REPOSITORY_ID)?;
    exact(
        string(value, "workflowPath", path)?,
        PROVENANCE_WORKFLOW,
        "provenance.workflowPath",
        path,
    )?;
    let run_id = positive(value, "runId", path)?;
    positive(value, "runAttempt", path)?;
    exact(
        string(value, "workflowRunUrl", path)?,
        &format!("{REPOSITORY}/actions/runs/{run_id}"),
        "provenance.workflowRunUrl",
        path,
    )
}

fn positive(value: &Map<String, Value>, field: &str, path: &Path) -> Result<i64> {
    let number = value.get(field).and_then(Value::as_i64).unwrap_or_default();
    if number > 0 {
        Ok(number)
    } else {
        bail!(
            "{} {field} must be a positive integer",
            display_relative(path)
        )
    }
}

fn check_compatibility(value: &Map<String, Value>, path: &Path) -> Result<()> {
    exact_keys(value, COMPATIBILITY_FIELDS, path)?;
    integer(value, "bootstrapApi", path, 1)?;
    integer(value, "pluginRuntimeApi", path, 1)?;
    exact(
        string(value, "transport", path)?,
        "stdio-newline-v1",
        "compatibility.transport",
        path,
    )?;
    exact(
        string(value, "mcpProtocol", path)?,
        "2024-11-05",
        "compatibility.mcpProtocol",
        path,
    )
}
