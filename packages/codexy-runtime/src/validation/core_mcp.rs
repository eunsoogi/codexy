use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::{json_array_strings, load_json};

const BOOTSTRAP: &str = "mcp/codexy_mcp_bootstrap.py";
const SOURCE_LAUNCHER: &str = "mcp/codexy-mcp-watcher.sh";

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
    let expected = super::mcp_required::shared_bootstrap_command("watcher");
    if command != expected {
        bail!(
            "{} watcher command must be the exact shared cross-platform MCP bootstrap",
            display_relative(&path)
        );
    }
    let launcher = plugin_root.join(SOURCE_LAUNCHER);
    let launcher_metadata = std::fs::symlink_metadata(&launcher).with_context(|| {
        format!(
            "core watcher launcher is missing: {}",
            display_relative(&launcher)
        )
    })?;
    if !launcher_metadata.file_type().is_file() || launcher_metadata.file_type().is_symlink() {
        bail!(
            "{} core watcher launcher must be a regular file",
            display_relative(&launcher)
        );
    }
    let bootstrap = plugin_root.join(BOOTSTRAP);
    let bootstrap_metadata = std::fs::symlink_metadata(&bootstrap).with_context(|| {
        format!(
            "core MCP bootstrap is missing: {}",
            display_relative(&bootstrap)
        )
    })?;
    if !bootstrap_metadata.file_type().is_file() || bootstrap_metadata.file_type().is_symlink() {
        bail!(
            "{} core MCP bootstrap must be a regular file",
            display_relative(&bootstrap)
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if launcher_metadata.permissions().mode() & 0o111 == 0 {
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
    let version = manifest
        .get("version")
        .and_then(Value::as_str)
        .context("core plugin manifest version must be a string")?;
    let expected = format!(
        concat!(
            "@echo off\n",
            "set \"plugin_root=%~dp0..\"\n",
            "set \"bundled_runtime=%plugin_root%\\runtime\\codexy-mcp-watcher-windows-x86_64.exe\"\n",
            "if exist \"%bundled_runtime%\" goto bundled_runtime\n",
            "where uvx >nul 2>&1\n",
            "if errorlevel 1 (\n",
            "  echo codexy-mcp-watcher requires uvx on PATH; install uv or provide a bundled runtime 1>&2\n",
            "  exit /b 127\n",
            ")\n",
            "set \"repo_root=%plugin_root%\\..\\..\"\n",
            "set \"runtime_source=%repo_root%\\packages\\getcodexy\"\n",
            "if exist \"%runtime_source%\\pyproject.toml\" goto local_source\n",
            "uvx --from getcodexy=={version} codexy-mcp-runtime watcher --plugin-root \"%plugin_root%\" -- %*\n",
            "exit /b %ERRORLEVEL%\n\n",
            ":local_source\n",
            "uvx --from \"%runtime_source%\" codexy-mcp-runtime watcher --plugin-root \"%plugin_root%\" -- %*\n",
            "exit /b %ERRORLEVEL%\n\n",
            ":bundled_runtime\n",
            "\"%bundled_runtime%\" %*\n",
            "exit /b %ERRORLEVEL%\n"
        ),
        version = version
    );
    if std::fs::read_to_string(&windows)
        .with_context(|| format!("reading {}", display_relative(&windows)))?
        != expected
    {
        bail!(
            "{} core watcher Windows launcher must preserve the bundled and source bootstrap contract",
            display_relative(&windows)
        );
    }
    let supported =
        super::manifest::supported_platforms(manifest, &super::manifest_path(plugin_root))?;
    super::runtime::check_core_watcher_artifacts(plugin_root, &supported)?;
    Ok(())
}
