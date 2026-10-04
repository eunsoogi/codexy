use std::path::Path;

use anyhow::{Result, bail};

use crate::paths::display_relative;

pub(super) fn check(plugin_root: &Path, server: &str) -> Result<()> {
    // Each server's .cmd file forwards to the shared dispatcher and must match the canonical bytes exactly.
    let path = plugin_root
        .join("mcp")
        .join(format!("codexy-mcp-{server}.cmd"));
    if !path.is_file() {
        bail!(
            "{} thin Windows MCP delegate missing",
            display_relative(&path)
        );
    }
    let expected = format!(
        concat!(
            "@echo off\n",
            "@rem Keep this server's public entrypoint on the shared native dispatcher.\n",
            "\"%~dp0codexy-mcp-devtools.exe\" {server} %*\n",
            "@rem Propagate the dispatcher exit status to the caller.\n",
            "exit /b %ERRORLEVEL%\n"
        ),
        server = server
    );
    let actual = std::fs::read(&path)?;
    if actual.starts_with(b"MZ") || actual != expected.as_bytes() {
        bail!(
            "{} must be the exact thin Windows MCP delegate for {server}",
            display_relative(&path)
        );
    }
    Ok(())
}
