//! Builds an explicitly registered pre-retirement core Watcher package for compatibility tests.

use std::path::Path;

use serde_json::{Value, json};

pub(crate) fn install_legacy_core_watcher_package(
    plugin_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    let version = manifest["version"]
        .as_str()
        .ok_or("legacy fixture core version must be a string")?
        .to_owned();
    manifest["mcpServers"] = json!("./.mcp.json");
    std::fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    let configuration = json!({
        "watcher": {
            "command": "uv",
            "args": [
                "run",
                "--no-project",
                "--script",
                "./mcp/codexy_mcp_bootstrap.py",
                "watcher",
                "--stdio"
            ],
            "cwd": "."
        }
    });
    std::fs::write(
        plugin_root.join(".mcp.json"),
        serde_json::to_vec_pretty(&configuration)?,
    )?;

    let mcp = plugin_root.join("mcp");
    std::fs::create_dir_all(&mcp)?;
    // These files are synthesized only for old explicit registrations after current-source deletion.
    std::fs::write(
        mcp.join("codexy_mcp_bootstrap.py"),
        "# synthetic legacy core MCP bootstrap\n",
    )?;
    let launcher = mcp.join("codexy-mcp-watcher.sh");
    std::fs::write(&launcher, "#!/bin/sh\nexit 0\n")?;
    set_executable(&launcher)?;
    std::fs::write(
        mcp.join("codexy-mcp-watcher.cmd"),
        windows_launcher(&version),
    )?;
    Ok(())
}

fn set_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn windows_launcher(version: &str) -> String {
    format!(
        concat!(
            "@echo off\n",
            "@rem Prefer the bundled runtime, then the checkout package, then the version-pinned release.\n",
            "@rem Return the selected launcher's exit status unchanged.\n",
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
    )
}
