use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::{json_array_strings, load_json};

const COMMAND: &[&str] = &["./mcp/codexy-mcp-watcher", "--stdio"];
const WINDOWS_LAUNCHER: &str = "@echo off\n\"%~dp0..\\runtime\\codexy-mcp-watcher-windows-x86_64.exe\" %*\nexit /b %ERRORLEVEL%\n";

pub(super) fn check(plugin_root: &Path, manifest: &Value) -> Result<()> {
    let path = super::manifest::mcp_config_path(plugin_root, manifest)?;
    let data = load_json(&path)?;
    if data.get("mcpServers").is_some() || data.get("mcp_servers").is_some() {
        bail!(
            "{} core MCP config must be a direct server map",
            display_relative(&path)
        );
    }
    let servers = data
        .as_object()
        .context("core MCP config must be an object")?;
    if servers.len() != 1 || !servers.contains_key("watcher") {
        bail!(
            "{} core MCP config must contain exactly watcher",
            display_relative(&path)
        );
    }
    let watcher = servers["watcher"]
        .as_object()
        .context("core watcher MCP entry must be an object")?;
    if watcher.get("cwd").and_then(Value::as_str) != Some(".") {
        bail!("{} watcher.cwd must be '.'", display_relative(&path));
    }
    let command = watcher
        .get("command")
        .and_then(Value::as_str)
        .map(|command| {
            let mut items = vec![command.to_owned()];
            items.extend(
                watcher
                    .get("args")
                    .and_then(|value| json_array_strings(Some(value)))
                    .unwrap_or_default(),
            );
            items
        })
        .context("core watcher.command must be a string")?;
    if command.iter().map(String::as_str).collect::<Vec<_>>() != COMMAND {
        bail!(
            "{} watcher command must be the exact core entrypoint",
            display_relative(&path)
        );
    }
    let launcher = plugin_root.join("mcp/codexy-mcp-watcher");
    let metadata = std::fs::symlink_metadata(&launcher).with_context(|| {
        format!(
            "core watcher launcher is missing: {}",
            display_relative(&launcher)
        )
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        bail!(
            "{} core watcher launcher must be a regular file",
            display_relative(&launcher)
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o111 == 0 {
            bail!(
                "{} core watcher launcher must be executable",
                display_relative(&launcher)
            );
        }
    }
    let windows = plugin_root.join("mcp/codexy-mcp-watcher.cmd");
    if !windows.is_file() {
        bail!(
            "{} core watcher Windows launcher is missing",
            display_relative(&windows)
        );
    }
    if std::fs::read_to_string(&windows)
        .with_context(|| format!("reading {}", display_relative(&windows)))?
        != WINDOWS_LAUNCHER
    {
        bail!(
            "{} core watcher Windows launcher must be the exact bundled runtime delegate",
            display_relative(&windows)
        );
    }
    let supported =
        super::manifest::supported_platforms(manifest, &super::manifest_path(plugin_root))?;
    super::runtime::check_core_watcher_artifacts(plugin_root, &supported)?;
    Ok(())
}
