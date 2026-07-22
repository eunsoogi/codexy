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
    for server in REQUIRED_RUNTIME_SERVERS {
        let wrapper_path = plugin_root.join("mcp").join(format!("codexy-mcp-{server}"));
        let wrapper_platforms = bundled_platforms(&wrapper_path)?;
        if wrapper_platforms != platforms {
            bail!(
                "{} bundled platforms for {server} must match supportedPlatforms: expected {:?}, got {:?}",
                display_relative(&wrapper_path),
                platforms,
                wrapper_platforms
            );
        }
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
                .join(super::runtime_binary::artifact_name(server, platform));
            if !runtime_path.is_file() {
                bail!(
                    "{} bundled MCP runtime missing for supported platform {platform}",
                    display_relative(&runtime_path)
                );
            }
            super::runtime_binary::check(&runtime_path, platform)?;
            if platform == "windows-x86_64" {
                super::runtime_binary::check_windows_entrypoint_copy(
                    plugin_root,
                    server,
                    &runtime_path,
                )?;
            }
        }
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
    for server in REQUIRED_RUNTIME_SERVERS {
        let entrypoint = plugin_root
            .join("mcp")
            .join(format!("codexy-mcp-{server}.exe"));
        if entrypoint.exists() {
            bail!(
                "{} must be generated only while assembling a Windows package",
                display_relative(&entrypoint)
            );
        }
    }
    Ok(())
}

fn check_runtime_build_matrix(platforms: &[String]) -> Result<()> {
    let path = crate::paths::repo_root()?.join(".github/workflows/plugin-runtime-binaries.yml");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", display_relative(&path)))?;
    for required in [
        "release:",
        "package-plugin:",
        "needs: build-runtime",
        "actions/download-artifact@v4",
        "pattern: codexy-mcp-runtimes-*",
        "dist/codexy-marketplace-plugin",
        "dist/codexy-marketplace-plugin.tar.gz",
        "--check-runtime-artifacts",
        "--check-hooks",
        "gh release upload",
    ] {
        if !text.contains(required) {
            bail!(
                "{} runtime package workflow must include {required:?}",
                display_relative(&path)
            );
        }
    }
    let package_validation_order = concat!(
        "--check-runtime-artifacts\n",
        "          scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-hooks\n",
        "          tar -C"
    );
    if !text.contains(package_validation_order) {
        bail!(
            "{} runtime package workflow must validate hooks before creating the archive",
            display_relative(&path)
        );
    }
    for forbidden in [
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
    for trigger in ["push:", "pull_request:"] {
        let trigger_text = workflow_trigger_block(&text, trigger).with_context(|| {
            format!(
                "{} runtime package workflow must include {trigger}",
                display_relative(&path)
            )
        })?;
        for required_path in ["plugins/codexy/**", "scripts/inspect-mcp-response"] {
            if !trigger_text.contains(required_path) {
                bail!(
                    "{} runtime package workflow {trigger} paths must include {required_path:?}",
                    display_relative(&path)
                );
            }
        }
    }
    for platform in platforms {
        if !text.contains(&format!("platform: {platform}")) {
            bail!(
                "{} runtime build matrix must cover supported platform {platform}",
                display_relative(&path)
            );
        }
        for server in REQUIRED_RUNTIME_SERVERS {
            let runtime_name = if platform == "windows-x86_64" {
                format!("codexy-mcp-{server}-$env:PLATFORM.exe")
            } else {
                format!("codexy-mcp-{server}-${{PLATFORM}}.bin")
            };
            if !text.contains(&runtime_name) {
                bail!(
                    "{} runtime build matrix must package {runtime_name}",
                    display_relative(&path)
                );
            }
        }
    }
    Ok(())
}

fn workflow_trigger_block<'a>(text: &'a str, trigger: &str) -> Option<&'a str> {
    let start = text.find(trigger)?;
    let rest = &text[start..];
    let end = rest
        .match_indices("\n  ")
        .find_map(|(index, _)| {
            let next = &rest[index + 3..];
            (!next.starts_with(' ')).then_some(index)
        })
        .unwrap_or(rest.len());
    Some(&rest[..end])
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
