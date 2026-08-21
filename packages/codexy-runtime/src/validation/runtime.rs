#[path = "runtime_binary.rs"]
mod runtime_binary;

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::manifest::{load_manifest, supported_platforms};
use crate::validation::manifest_path;

const REQUIRED_RUNTIME_SERVERS: &[&str] = &["lsp", "codegraph"];
const GENERATED_SOURCE_DIRS: &[&str] = &["bin", "runtime"];

pub(super) fn check_source_contract(plugin_root: &Path, manifest: &Value) -> Result<()> {
    check_no_source_runtime_artifacts(plugin_root)?;
    let path = manifest_path(plugin_root);
    let platforms = supported_platforms(manifest, &path)?;
    let release = plugin_root.join("runtime-release.json");
    let candidate = plugin_root.join("runtime-candidate.json");
    if release.exists() {
        crate::validation::runtime_release_contract::check(plugin_root, &platforms)?;
    } else if candidate.exists() || platforms != ["darwin-arm64", "linux-x86_64"] {
        bail!(
            "{} public-bootstrap source mode requires no runtime contracts and exactly darwin/linux support",
            display_relative(plugin_root)
        );
    }
    let wrapper_path = plugin_root.join("mcp/codexy-mcp-devtools");
    let wrapper_platforms = bundled_platforms(&wrapper_path)?;
    if wrapper_platforms != platforms {
        bail!(
            "{} bundled platforms must match supportedPlatforms: expected {:?}, got {:?}",
            display_relative(&wrapper_path),
            platforms,
            wrapper_platforms
        );
    }
    check_runtime_build_matrix(&platforms)?;
    crate::validation::release_publish_contract::check_snapshot_contract(&platforms)
}

pub(super) fn check_artifacts(plugin_root: &Path) -> Vec<String> {
    match load_manifest(plugin_root)
        .and_then(|manifest| check_packaged_runtime_artifacts(plugin_root, &manifest))
    {
        Ok(()) => Vec::new(),
        Err(error) => vec![error.to_string()],
    }
}

fn check_packaged_runtime_artifacts(plugin_root: &Path, manifest: &Value) -> Result<()> {
    if plugin_root.join("bin").exists() {
        bail!(
            "{} must not contain generated MCP runtimes or wrappers",
            display_relative(&plugin_root.join("bin"))
        );
    }
    let path = manifest_path(plugin_root);
    let platforms = supported_platforms(manifest, &path)?;
    for server in REQUIRED_RUNTIME_SERVERS {
        for platform in &platforms {
            let runtime_path = plugin_root
                .join("runtime")
                .join(runtime_binary::artifact_name(server, platform));
            if !runtime_path.is_file() {
                bail!(
                    "{} bundled MCP runtime missing for supported platform {platform}",
                    display_relative(&runtime_path)
                );
            }
            runtime_binary::check(&runtime_path, platform)?;
        }
    }
    for platform in &platforms {
        runtime_binary::check_distinct_server_runtimes(plugin_root, platform)?;
    }
    if platforms
        .iter()
        .any(|platform| platform == "windows-x86_64")
    {
        runtime_binary::check_windows_dispatcher(plugin_root)?;
    }
    Ok(())
}

fn check_no_source_runtime_artifacts(plugin_root: &Path) -> Result<()> {
    for dir in GENERATED_SOURCE_DIRS {
        let path = plugin_root.join(dir);
        if path.exists() {
            bail!(
                "{} must not be tracked in the source plugin tree",
                display_relative(&path)
            );
        }
    }
    Ok(())
}

fn check_runtime_build_matrix(platforms: &[String]) -> Result<()> {
    let root = crate::paths::repo_root()?;
    let path = root.join(".github/workflows/plugin-runtime-binaries.yml");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", display_relative(&path)))?;
    let legacy = ["darwin-arm64", "linux-x86_64"];
    if platforms != legacy {
        bail!(
            "{} source marketplace must retain the darwin/linux public-bootstrap platforms",
            display_relative(&path),
        );
    }
    for required in [
        "verify-selected-package:",
        "Download and verify selected immutable bytes",
        "scripts/download-selected-runtime-package dist/selected.tar.gz",
        "Assemble state-aware marketplace package without rebuilding",
        ".agents/plugins/runtime-activation.json",
        "scripts/materialize-runtime-release-archive",
        "public-release",
        "dist/codexy-marketplace-plugin",
        "dist/codexy-marketplace-plugin.tar.gz",
        "scripts/inspect-release-archive",
        "verify-windows-selected-candidate:",
        "Verify immutable native Windows candidate bytes",
    ] {
        if !text.contains(required) {
            bail!(
                "{} immutable runtime package workflow must include {required:?}",
                display_relative(&path)
            );
        }
    }
    for forbidden in [
        "cargo build",
        "build-runtime",
        "codexy-mcp-lsp-${PLATFORM}.bin",
        "Publish generated marketplace snapshot",
        "MARKETPLACE_BRANCH",
        "dist/marketplace-root",
        "git -C \"$marketplace_root\" push --force origin \"$MARKETPLACE_BRANCH\"",
    ] {
        if text.contains(forbidden) {
            bail!(
                "{} runtime package workflow must not require {forbidden:?}",
                display_relative(&path)
            );
        }
    }
    Ok(())
}

pub(super) fn check_selected_runtime_source_helper(root: &Path) -> Result<()> {
    let path = root.join("scripts/download-selected-runtime-package");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", display_relative(&path)))?;
    for required in [
        "curl --fail --location \"$url\"",
        "command -v sha256sum",
        "shasum -a 256",
        "runtime-release-receipt.json",
        "legacy_release=plugins/codexy-devtools/runtime-release.json",
        "scripts/download-runtime-staging-artifact staging",
        "test \"$url\" = \"$expected_url\"",
        "test \"$(digest_file \"$output\")\" = \"$digest\"",
    ] {
        if !text.contains(required) {
            bail!(
                "{} selected runtime source helper must include {required:?}",
                display_relative(&path)
            );
        }
    }
    Ok(())
}

fn bundled_platforms(wrapper_path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(wrapper_path)
        .with_context(|| format!("reading {}", display_relative(wrapper_path)))?;
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix("bundled_platforms=\""))
        .and_then(|line| line.strip_suffix('"'))
        .with_context(|| {
            format!(
                "{} must declare bundled_platforms",
                display_relative(wrapper_path)
            )
        })?;
    Ok(line.split_whitespace().map(ToOwned::to_owned).collect())
}
