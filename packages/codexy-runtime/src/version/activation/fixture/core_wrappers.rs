use std::path::Path;

use anyhow::Result;

pub(super) const PATHS: [&str; 2] = [
    "plugins/codexy/mcp/codexy-mcp-watcher.sh",
    "plugins/codexy/mcp/codexy-mcp-watcher.cmd",
];

pub(super) fn write(root: &Path, version: &str) -> Result<()> {
    std::fs::create_dir_all(root.join("plugins/codexy/mcp"))?;
    super::write(
        root,
        PATHS[0],
        format!(
            "#!/bin/sh\nexec uvx --from getcodexy=={version} codexy-mcp-runtime watcher --plugin-root \"$plugin_root\" -- \"$@\"\n"
        ),
    )?;
    super::write(
        root,
        PATHS[1],
        format!(
            "@echo off\nuvx --from getcodexy=={version} codexy-mcp-runtime watcher --plugin-root \"%~dp0..\" -- %*\nexit /b %ERRORLEVEL%\n"
        ),
    )?;
    Ok(())
}
