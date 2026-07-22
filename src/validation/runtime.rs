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
    crate::validation::runtime_release_contract::check(plugin_root, &platforms)?;
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
                .join(format!("codexy-mcp-{server}-{platform}.bin"));
            if !runtime_path.is_file() {
                bail!(
                    "{} bundled MCP runtime missing for supported platform {platform}",
                    display_relative(&runtime_path)
                );
            }
            check_runtime_binary_signature(&runtime_path, platform)?;
            check_runtime_executable(&runtime_path)?;
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
    Ok(())
}

fn check_runtime_binary_signature(runtime_path: &Path, platform: &str) -> Result<()> {
    let bytes = std::fs::read(runtime_path)
        .with_context(|| format!("reading {}", display_relative(runtime_path)))?;
    match platform {
        "linux-x86_64" if bytes.starts_with(b"\x7fELF") => Ok(()),
        "darwin-arm64"
            if bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
                || bytes.starts_with(&[0xfe, 0xed, 0xfa, 0xcf]) =>
        {
            Ok(())
        }
        "linux-x86_64" | "darwin-arm64" => bail!(
            "{} bundled MCP runtime has invalid binary format for {platform}",
            display_relative(runtime_path)
        ),
        _ => Ok(()),
    }
}

fn check_runtime_executable(runtime_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        let mode = runtime_path
            .metadata()
            .with_context(|| format!("reading {}", display_relative(runtime_path)))?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            bail!(
                "{} bundled MCP runtime must be executable",
                display_relative(runtime_path)
            );
        }
    }
    #[cfg(not(unix))]
    let _ = runtime_path;
    Ok(())
}

fn check_runtime_build_matrix(platforms: &[String]) -> Result<()> {
    let path = crate::paths::repo_root()?.join(".github/workflows/plugin-runtime-binaries.yml");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", display_relative(&path)))?;
    let selected = ["darwin-arm64", "linux-x86_64"];
    if platforms != selected {
        bail!(
            "{} immutable runtime package must retain platforms {:?}",
            display_relative(&path),
            selected
        );
    }
    for required in [
        "verify-selected-package:",
        "Download and verify selected immutable bytes",
        "sha256sum dist/selected.tar.gz",
        "Assemble state-aware marketplace package without rebuilding",
        "candidate-proven",
        "runtime-candidate.json",
        "payloadManifestSha256",
        "test ! -e \"$candidate/runtime-release.json\"",
        "for platform in darwin-arm64 linux-x86_64",
        "dist/codexy-marketplace-plugin",
        "dist/codexy-marketplace-plugin.tar.gz",
        "scripts/inspect-release-archive",
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
        "actions/download-artifact",
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
