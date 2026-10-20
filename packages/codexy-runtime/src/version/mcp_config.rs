//! Verifies the optional legacy core and required devtools MCP package contracts.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::mcp_required::is_shared_bootstrap_entry;

const CORE_MANIFEST: &str = "plugins/codexy/.codex-plugin/plugin.json";
const CORE_PLUGIN: &str = "plugins/codexy";
const CORE_CONFIG: &str = ".mcp.json";
const DEVTOOLS_CONFIG: &str = "plugins/codexy-devtools/.mcp.json";

pub(super) fn check_at(root: &Path) -> Result<()> {
    let core_manifest_path = root.join(CORE_MANIFEST);
    // Preserve pair-based checks for reduced snapshots that predate plugin manifests.
    match fs::symlink_metadata(&core_manifest_path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return check_legacy_pair(root);
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "inspecting core manifest {}",
                    display_relative(&core_manifest_path)
                )
            });
        }
    }
    let core_manifest = super::load_json(&core_manifest_path)?;
    if core_manifest.get("mcp_servers").is_some() {
        bail!(
            "{} core plugin manifest must use the mcpServers field",
            display_relative(&core_manifest_path)
        );
    }
    match core_manifest.get("mcpServers") {
        None => reject_present(
            &root.join(CORE_PLUGIN).join(CORE_CONFIG),
            "core MCP config is present without a manifest declaration",
        )?,
        Some(declaration) => {
            let plugin_root = root.join(CORE_PLUGIN);
            let configured = declaration
                .as_str()
                .filter(|value| !value.is_empty())
                .with_context(|| {
                    format!(
                        "{} mcpServers must be a path string",
                        display_relative(&core_manifest_path)
                    )
                })?;
            let configured_path =
                resolve_plugin_path(&plugin_root, Path::new(configured), &core_manifest_path)?;
            let default_path =
                resolve_plugin_path(&plugin_root, Path::new(CORE_CONFIG), &core_manifest_path)?;
            // A legacy manifest may point elsewhere inside the package, but it cannot leave an unregistered default config behind.
            if configured_path != default_path {
                reject_present(
                    &plugin_root.join(CORE_CONFIG),
                    "unexpected residual core MCP config",
                )?;
            }
            check_config(&configured_path, &["watcher"])?;
        }
    }
    check_config(&root.join(DEVTOOLS_CONFIG), &["lsp", "codegraph"])
}

fn check_legacy_pair(root: &Path) -> Result<()> {
    let core_config = root.join(CORE_PLUGIN).join(CORE_CONFIG);
    let devtools_config = root.join(DEVTOOLS_CONFIG);
    let present = [core_config.is_file(), devtools_config.is_file()]
        .into_iter()
        .filter(|exists| *exists)
        .count();
    if present == 0 {
        return Ok(());
    }
    if present != 2 {
        bail!("MCP bootstrap configuration is only partially present");
    }
    check_config(&core_config, &["watcher"])?;
    check_config(&devtools_config, &["lsp", "codegraph"])
}

fn check_config(path: &Path, servers: &[&str]) -> Result<()> {
    let value = super::load_json(path)?;
    let object = value.as_object().context("MCP config must be an object")?;
    if object.len() != servers.len()
        || servers.iter().any(|server| {
            !is_shared_bootstrap_entry(object.get(*server).and_then(Value::as_object), server)
        })
    {
        bail!(
            "{} must use the shared metadata-driven MCP bootstrap",
            display_relative(path)
        );
    }
    Ok(())
}

fn resolve_plugin_path(plugin_root: &Path, configured: &Path, manifest: &Path) -> Result<PathBuf> {
    if configured.is_absolute() {
        bail!(
            "{} mcpServers must be plugin-relative",
            display_relative(manifest)
        );
    }
    let canonical_root = plugin_root
        .canonicalize()
        .with_context(|| format!("resolving plugin root {}", display_relative(plugin_root)))?;
    let candidate = plugin_root.join(configured);
    // Resolve existing symlinks, or resolve the parent for missing files just as source validation does.
    let resolved = match candidate.canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = candidate
                .parent()
                .context("MCP config path has no parent")?;
            let file_name = candidate
                .file_name()
                .context("MCP config path has no filename")?;
            parent
                .canonicalize()
                .map(|parent| parent.join(file_name))
                .with_context(|| {
                    format!("resolving MCP config parent {}", display_relative(parent))
                })?
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!("resolving MCP config path {}", display_relative(&candidate))
            });
        }
    };
    if !resolved.starts_with(&canonical_root) {
        bail!(
            "{} mcpServers must stay inside the plugin root",
            display_relative(manifest)
        );
    }
    Ok(resolved)
}

fn reject_present(path: &Path, reason: &str) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => bail!("{} {reason}", display_relative(path)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("inspecting {}", display_relative(path))),
    }
}
