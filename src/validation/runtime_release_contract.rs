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
    exact_keys(
        root,
        &[
            "schema",
            "state",
            "source",
            "artifact",
            "compatibility",
            "platforms",
        ],
        &path,
    )?;
    exact(string(root, "schema", &path)?, SCHEMA, "schema", &path)?;
    let state = string(root, "state", &path)?;
    check_source(object_field(root, "source", &path)?, state, &path)?;
    check_artifact(object_field(root, "artifact", &path)?, state, &path)?;
    check_compatibility(object_field(root, "compatibility", &path)?, &path)?;
    check_platforms(
        object_field(root, "platforms", &path)?,
        supported,
        state,
        &path,
    )?;
    if state == "candidate-proven" {
        crate::validation::runtime_candidate_manifest::check(plugin_root, root, &path)?;
    }
    Ok(())
}

fn check_source(source: &Map<String, Value>, state: &str, path: &Path) -> Result<()> {
    exact_keys(source, &["repository", "commit"], path)?;
    exact(
        string(source, "repository", path)?,
        REPOSITORY,
        "source.repository",
        path,
    )?;
    let commit = string(source, "commit", path)?;
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!(
            "{} source.commit must be a lowercase 40-character commit",
            display_relative(path)
        );
    }
    if state == "legacy-public" {
        exact(commit, LEGACY_COMMIT, "source.commit", path)?;
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
    exact(
        url,
        &format!("{REPOSITORY}/releases/download/{tag}/codexy-marketplace-plugin.tar.gz"),
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
            let slug = tag.strip_prefix("runtime-candidate-").unwrap_or_default();
            if !slug.is_empty()
                && slug
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            {
                Ok(())
            } else {
                bail!(
                    "{} artifact.tag must have a safe runtime-candidate slug",
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
                    legacy_digest(platform, server),
                    "platform digest",
                    path,
                )?;
            } else {
                exact(
                    string(binary, "path", path)?,
                    &format!("runtime/codexy-mcp-{server}-{platform}.bin"),
                    "candidate runtime path",
                    path,
                )?;
            }
        }
    }
    Ok(())
}

fn legacy_digest(platform: &str, server: &str) -> &'static str {
    match (platform, server) {
        ("darwin-arm64", "lsp") => {
            "0a6eda4597abd517f61c230aeabf6e81666e619aaeecc324275a2d26cdc70706"
        }
        ("darwin-arm64", "codegraph") => {
            "f6ac5faee4261167c7783e6cd69a0610b3cbf4abcbf5944d395213868d356dc6"
        }
        ("linux-x86_64", "lsp") => {
            "7504edd84efa75c346c478a6bff6076950b8339eaf95472a9754147ae6529663"
        }
        ("linux-x86_64", "codegraph") => {
            "218c5d896f912333c38c74034f6df6f0e54a70cf1fc418e1b04f808f29bea2b2"
        }
        _ => unreachable!("validated runtime inventory"),
    }
}
