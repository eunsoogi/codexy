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

use super::{core, legacy::legacy_digest};

const SCHEMA: &str = "codexy-runtime-release/v1";
const REPOSITORY: &str = "https://github.com/eunsoogi/codexy";
const LEGACY_COMMIT: &str = "6890b3089dcffc2293f8f63b761e33562250eac6";
const LEGACY_TAG: &str = "v1.2.2";
const LEGACY_ARCHIVE_SHA: &str = "6cd61a3472d9a70d818251f1abd3e264e27a59ade4a05929014afc1c9de96293";
const LEGACY_MANIFEST_SHA: &str =
    "0056e191fa5d837f770bc5e5f8a2be855b9252e299847522b0d88e6b186b42f2";
const LEGACY_PLATFORMS: &[&str] = &["darwin-arm64", "linux-x86_64"];
#[rustfmt::skip]
const PROVENANCE_FIELDS: &[&str] = &["repositoryId", "workflowPath", "runId", "runAttempt", "workflowRunUrl"];
const PROVENANCE_WORKFLOW: &str = ".github/workflows/runtime-candidate.yml";
const REPOSITORY_ID: i64 = 1_269_350_143;
#[rustfmt::skip]
const BASE_FIELDS: &[&str] = &["schema", "state", "source", "artifact", "compatibility", "platforms"];
#[rustfmt::skip]
const CANDIDATE_CORE_FIELDS: &[&str] = &["schema", "state", "source", "artifact", "compatibility", "platforms", "classes"];
#[rustfmt::skip]
const SOURCE_FIELDS: &[&str] = &["schema", "state", "source", "artifact", "provenance", "compatibility", "platforms"];
#[rustfmt::skip]
const SOURCE_CORE_FIELDS: &[&str] = &["schema", "state", "source", "artifact", "provenance", "compatibility", "platforms", "classes"];
#[rustfmt::skip]
const COMPATIBILITY_FIELDS: &[&str] = &["bootstrapApi", "pluginRuntimeApi", "transport", "mcpProtocol"];
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
    core::check_source(source, state, REPOSITORY, LEGACY_COMMIT, &path)?;
    check_artifact(object_field(root, "artifact", &path)?, state, &path)?;
    check_compatibility(object_field(root, "compatibility", &path)?, &path)?;
    if state == "source-selected" {
        check_provenance(object_field(root, "provenance", &path)?, &path)?;
    }
    check_platforms(
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
    let outer = digest(string(artifact, "sha256", path)?, "artifact.sha256", path)?;
    let payload = digest(
        string(artifact, "payloadManifestSha256", path)?,
        "artifact.payloadManifestSha256",
        path,
    )?;
    if state == "legacy-public" {
        exact(tag, LEGACY_TAG, "artifact.tag", path)?;
        exact(outer, LEGACY_ARCHIVE_SHA, "artifact.sha256", path)?;
        exact(
            payload,
            LEGACY_MANIFEST_SHA,
            "artifact.payloadManifestSha256",
            path,
        )?;
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

#[rustfmt::skip]
fn check_platforms(value: &Map<String, Value>, supported: &[String], state: &str, path: &Path) -> Result<()> {
    let legacy = LEGACY_PLATFORMS.iter().map(|item| (*item).to_owned()).collect::<Vec<_>>();
    let expected = if matches!(state, "legacy-public" | "source-selected") { legacy.clone() } else { supported.to_vec() };
    if state == "legacy-public" && supported != legacy.as_slice() { bail!("{} legacy-public state must retain the selected two-platform baseline", display_relative(path)); }
    if value.keys().cloned().collect::<Vec<_>>() != expected { bail!("{} platforms must exactly be {:?}", display_relative(path), expected); }
    for platform in &expected {
        let inventory = object_field(value, platform, path)?;
        exact_keys(inventory, &SERVERS, path)?;
        for server in SERVERS {
            let binary = object_field(inventory, server, path)?;
            let fields = if state == "legacy-public" { &["sha256"][..] } else { &["path", "sha256"][..] };
            exact_keys(binary, fields, path)?;
            let binary_digest = digest(string(binary, "sha256", path)?, "platform digest", path)?;
            if state == "legacy-public" {
                exact(binary_digest, legacy_digest(platform, server).ok_or_else(|| anyhow::anyhow!("unsupported legacy runtime inventory: {platform}/{server}"))?, "platform digest", path)?;
            } else {
                let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
                exact(string(binary, "path", path)?, &format!("runtime/codexy-mcp-{server}-{platform}.{extension}"), "candidate runtime path", path)?;
            }
        }
    }
    Ok(())
}
