use std::path::Path;

use anyhow::{Result, bail};

use crate::paths::display_relative;

pub(super) fn check(plugin_root: &Path, server: &str) -> Result<()> {
    let path = plugin_root
        .join("mcp")
        .join(format!("codexy-mcp-{server}.cmd"));
    if !path.is_file() {
        bail!(
            "{} thin Windows MCP delegate missing",
            display_relative(&path)
        );
    }
    let expected =
        format!("@echo off\n\"%~dp0codexy-mcp-devtools.exe\" {server} %*\nexit /b %ERRORLEVEL%\n");
    let actual = std::fs::read(&path)?;
    if actual.starts_with(b"MZ") || actual != expected.as_bytes() {
        bail!(
            "{} must be the exact thin Windows MCP delegate for {server}",
            display_relative(&path)
        );
    }
    Ok(())
}
