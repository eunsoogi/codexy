pub(crate) mod core;
mod legacy;

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
use legacy::legacy_digest;

const SCHEMA: &str = "codexy-runtime-release/v1";
const REPOSITORY: &str = "https://github.com/eunsoogi/codexy";
const LEGACY_COMMIT: &str = "6890b3089dcffc2293f8f63b761e33562250eac6";
const LEGACY_TAG: &str = "v1.2.2";
const LEGACY_ARCHIVE_SHA: &str = "6cd61a3472d9a70d818251f1abd3e264e27a59ade4a05929014afc1c9de96293";
const LEGACY_MANIFEST_SHA: &str =
    "0056e191fa5d837f770bc5e5f8a2be855b9252e299847522b0d88e6b186b42f2";
const LEGACY_PLATFORMS: &[&str] = &["darwin-arm64", "linux-x86_64"];

pub(super) fn check(plugin_root: &Path, supported: &[String]) -> Result<()> {
    let path = plugin_root.join("runtime-release.json");
    let contract = load_json(&path)?;
    let root = object(&contract, "root", &path)?;
    exact(string(root, "schema", &path)?, SCHEMA, "schema", &path)?;
    let state = string(root, "state", &path)?;
    let core_aware =
        root.contains_key("classes") || object_field(root, "source", &path)?.contains_key("tree");
    let fields = if state == "candidate-proven" && core_aware {
        &[
            "schema",
            "state",
            "source",
            "artifact",
            "compatibility",
            "platforms",
            "classes",
        ][..]
    } else {
        &[
            "schema",
            "state",
            "source",
            "artifact",
            "compatibility",
            "platforms",
        ][..]
    };
    exact_keys(root, fields, &path)?;
    core::check_source(
        object_field(root, "source", &path)?,
        state,
        REPOSITORY,
        LEGACY_COMMIT,
        &path,
    )?;
    check_artifact(object_field(root, "artifact", &path)?, state, &path)?;
    check_compatibility(object_field(root, "compatibility", &path)?, &path)?;
    check_platforms(
        object_field(root, "platforms", &path)?,
        supported,
        state,
        &path,
    )?;
    if state == "candidate-proven" && core_aware {
        core::check(
            object_field(root, "classes", &path)?,
            object_field(root, "source", &path)?,
            object_field(root, "platforms", &path)?,
            &path,
        )?;
        core::check_manifest(plugin_root, root, &path)?;
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
    let url = string(artifact, "url", path)?;
    let asset = if state == "candidate-proven" {
        "codexy-runtime-package.tar.gz"
    } else {
        "codexy-marketplace-plugin.tar.gz"
    };
    exact(
        url,
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
    match state {
        "legacy-public" => {
            exact(tag, LEGACY_TAG, "artifact.tag", path)?;
            exact(outer, LEGACY_ARCHIVE_SHA, "artifact.sha256", path)?;
            exact(
                payload,
                LEGACY_MANIFEST_SHA,
                "artifact.payloadManifestSha256",
                path,
            )
        }
        "candidate-proven" => Ok(()),
        _ => bail!(
            "{} state must be legacy-public or candidate-proven",
            display_relative(path)
        ),
    }
}

fn check_tag(tag: &str, state: &str, path: &Path) -> Result<()> {
    match state {
        "legacy-public" => exact(tag, LEGACY_TAG, "artifact.tag", path),
        "candidate-proven" => {
            let version = tag.strip_prefix('v').unwrap_or_default();
            if version.split('.').count() == 3
                && version.split('.').all(|component| {
                    !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
                })
            {
                Ok(())
            } else {
                bail!(
                    "{} artifact.tag must be a version-only vMAJOR.MINOR.PATCH tag",
                    display_relative(path)
                )
            }
        }
        _ => bail!(
            "{} state must be legacy-public or candidate-proven",
            display_relative(path)
        ),
    }
}

fn check_compatibility(value: &Map<String, Value>, path: &Path) -> Result<()> {
    exact_keys(
        value,
        &[
            "bootstrapApi",
            "pluginRuntimeApi",
            "transport",
            "mcpProtocol",
        ],
        path,
    )?;
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

fn check_platforms(
    value: &Map<String, Value>,
    supported: &[String],
    state: &str,
    path: &Path,
) -> Result<()> {
    let expected = supported.to_vec();
    if state == "legacy-public" && expected != LEGACY_PLATFORMS {
        bail!(
            "{} legacy-public state must retain the selected two-platform baseline",
            display_relative(path)
        );
    }
    let keys = value.keys().cloned().collect::<Vec<_>>();
    if keys != expected {
        bail!(
            "{} platforms must exactly be {:?}",
            display_relative(path),
            expected
        );
    }
    for platform in supported {
        let inventory = object_field(value, platform, path)?;
        exact_keys(inventory, &["lsp", "codegraph"], path)?;
        for server in ["lsp", "codegraph"] {
            let binary = object_field(inventory, server, path)?;
            let fields = if state == "legacy-public" {
                &["sha256"][..]
            } else {
                &["path", "sha256"][..]
            };
            exact_keys(binary, fields, path)?;
            let digest = digest(
                string(binary, "sha256", path)?,
                &format!("platforms.{platform}.{server}.sha256"),
                path,
            )?;
            if state == "legacy-public" {
                exact(
                    digest,
                    legacy_digest(platform, server).ok_or_else(|| {
                        anyhow::anyhow!("unsupported legacy runtime inventory: {platform}/{server}")
                    })?,
                    "platform digest",
                    path,
                )?;
            } else {
                exact(
                    string(binary, "path", path)?,
                    &format!(
                        "runtime/codexy-mcp-{server}-{platform}.{}",
                        if platform == "windows-x86_64" {
                            "exe"
                        } else {
                            "bin"
                        }
                    ),
                    "candidate runtime path",
                    path,
                )?;
            }
        }
    }
    Ok(())
}
