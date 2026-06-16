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
    for required in [
        "release:",
        "package-plugin:",
        "needs: build-runtime",
        "actions/download-artifact@v4",
        "pattern: codexy-mcp-runtimes-*",
        "dist/codexy-marketplace-plugin",
        "dist/codexy-marketplace-plugin.tar.gz",
        "--check-runtime-artifacts",
        "gh release upload",
        "codexy-main",
        "concurrency:",
        "cancel-in-progress: true",
        "Verify main dogfood package is current",
        "repos/${GITHUB_REPOSITORY}/git/ref/heads/main",
        "steps.main-package-head.outputs.current == 'true'",
        "Create main dogfood package release",
        "Upload main dogfood package",
    ] {
        if !text.contains(required) {
            bail!(
                "{} runtime package workflow must include {required:?}",
                display_relative(&path)
            );
        }
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
    for platform in platforms {
        if !text.contains(&format!("platform: {platform}")) {
            bail!(
                "{} runtime build matrix must cover supported platform {platform}",
                display_relative(&path)
            );
        }
        for server in REQUIRED_RUNTIME_SERVERS {
            let runtime_name = format!("codexy-mcp-{server}-${{PLATFORM}}.bin");
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
