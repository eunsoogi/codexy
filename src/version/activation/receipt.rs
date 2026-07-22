mod fields;

use anyhow::{Result, bail};
use serde_json::{Map, Value, json};

use fields::*;

const RECEIPT_SCHEMA: &str = "codexy-runtime-candidate-receipt/v1";
const CANDIDATE_SCHEMA: &str = "codexy-runtime-candidate/v1";
const RELEASE_SCHEMA: &str = "codexy-runtime-release/v1";
const REPOSITORY: &str = "https://github.com/eunsoogi/codexy";
const REPOSITORY_ID: i64 = 1_269_350_143;
const WORKFLOW_PATH: &str = ".github/workflows/runtime-candidate.yml";
const PLATFORMS: [&str; 2] = ["darwin-arm64", "linux-x86_64"];
const SERVERS: [&str; 2] = ["lsp", "codegraph"];

pub(super) fn activation_from_receipt(receipt: &Value) -> Result<(Value, Value)> {
    let root = object(receipt, "candidate receipt")?;
    exact_keys(
        root,
        &["schema", "candidate", "artifact", "provenance"],
        "candidate receipt",
    )?;
    exact(
        string(root, "schema", "candidate receipt")?,
        RECEIPT_SCHEMA,
        "candidate receipt schema",
    )?;
    let candidate = object_field(root, "candidate", "candidate receipt")?;
    let artifact = object_field(root, "artifact", "candidate receipt")?;
    validate_provenance(object_field(root, "provenance", "candidate receipt")?)?;
    validate_candidate(candidate)?;
    let tag = string(
        object_field(candidate, "artifact", "candidate")?,
        "tag",
        "candidate artifact",
    )?;
    validate_artifact(artifact, tag)?;
    let source = object_field(candidate, "source", "candidate")?;
    let compatibility = object_field(candidate, "compatibility", "candidate")?;
    let platforms = object_field(candidate, "platforms", "candidate")?;
    let release_platforms = release_platforms(platforms)?;
    let release = json!({
        "schema": RELEASE_SCHEMA,
        "state": "candidate-proven",
        "source": source,
        "artifact": artifact,
        "compatibility": compatibility,
        "platforms": release_platforms,
    });
    Ok((release, Value::Object(candidate.clone())))
}

fn release_platforms(platforms: &Map<String, Value>) -> Result<Value> {
    let entries = PLATFORMS
        .into_iter()
        .map(|platform| {
            let inventory = object_field(platforms, platform, "candidate platforms")?;
            let binaries = SERVERS
                .into_iter()
                .map(|server| {
                    let binary = object_field(inventory, server, "candidate platform")?;
                    Ok((server.to_owned(), Value::Object(binary.clone())))
                })
                .collect::<Result<Map<_, _>>>()?;
            Ok((platform.to_owned(), Value::Object(binaries)))
        })
        .collect::<Result<Map<_, _>>>()?;
    Ok(Value::Object(entries))
}

fn validate_candidate(candidate: &Map<String, Value>) -> Result<()> {
    exact_keys(
        candidate,
        &["schema", "source", "artifact", "compatibility", "platforms"],
        "candidate",
    )?;
    exact(
        string(candidate, "schema", "candidate")?,
        CANDIDATE_SCHEMA,
        "candidate schema",
    )?;
    let source = object_field(candidate, "source", "candidate")?;
    exact_keys(source, &["repository", "commit"], "candidate source")?;
    exact(
        string(source, "repository", "candidate source")?,
        REPOSITORY,
        "candidate repository",
    )?;
    commit(string(source, "commit", "candidate source")?)?;
    let artifact = object_field(candidate, "artifact", "candidate")?;
    exact_keys(artifact, &["tag"], "candidate artifact")?;
    tag(string(artifact, "tag", "candidate artifact")?)?;
    let compatibility = object_field(candidate, "compatibility", "candidate")?;
    exact_keys(
        compatibility,
        &[
            "bootstrapApi",
            "pluginRuntimeApi",
            "transport",
            "mcpProtocol",
        ],
        "candidate compatibility",
    )?;
    if compatibility.get("bootstrapApi") != Some(&json!(1))
        || compatibility.get("pluginRuntimeApi") != Some(&json!(1))
    {
        bail!("candidate compatibility APIs must be 1");
    }
    exact(
        string(compatibility, "transport", "candidate compatibility")?,
        "stdio-newline-v1",
        "candidate transport",
    )?;
    exact(
        string(compatibility, "mcpProtocol", "candidate compatibility")?,
        "2024-11-05",
        "candidate protocol",
    )?;
    let platforms = object_field(candidate, "platforms", "candidate")?;
    exact_keys(platforms, &PLATFORMS, "candidate platforms")?;
    for platform in PLATFORMS {
        let inventory = object_field(platforms, platform, "candidate platforms")?;
        exact_keys(inventory, &SERVERS, "candidate platform")?;
        for server in SERVERS {
            let binary = object_field(inventory, server, "candidate platform")?;
            exact_keys(binary, &["path", "sha256"], "candidate binary")?;
            binary_path(
                string(binary, "path", "candidate binary")?,
                server,
                platform,
            )?;
            digest(string(binary, "sha256", "candidate binary")?)?;
        }
    }
    Ok(())
}

fn validate_artifact(artifact: &Map<String, Value>, tag_value: &str) -> Result<()> {
    exact_keys(
        artifact,
        &["url", "sha256", "payloadManifestSha256"],
        "candidate artifact proof",
    )?;
    exact(
        string(artifact, "url", "candidate artifact proof")?,
        &format!("{REPOSITORY}/releases/download/{tag_value}/codexy-marketplace-plugin.tar.gz"),
        "candidate artifact URL",
    )?;
    digest(string(artifact, "sha256", "candidate artifact proof")?)?;
    digest(string(
        artifact,
        "payloadManifestSha256",
        "candidate artifact proof",
    )?)
}

fn validate_provenance(provenance: &Map<String, Value>) -> Result<()> {
    exact_keys(
        provenance,
        &[
            "repositoryId",
            "workflowPath",
            "runId",
            "runAttempt",
            "workflowRunUrl",
        ],
        "candidate provenance",
    )?;
    if provenance.get("repositoryId") != Some(&json!(REPOSITORY_ID)) {
        bail!("candidate provenance repositoryId is not canonical");
    }
    exact(
        string(provenance, "workflowPath", "candidate provenance")?,
        WORKFLOW_PATH,
        "candidate workflow path",
    )?;
    let run_id = positive_integer(provenance, "runId", "candidate provenance")?;
    positive_integer(provenance, "runAttempt", "candidate provenance")?;
    exact(
        string(provenance, "workflowRunUrl", "candidate provenance")?,
        &format!("{REPOSITORY}/actions/runs/{run_id}"),
        "candidate workflow run URL",
    )
}
