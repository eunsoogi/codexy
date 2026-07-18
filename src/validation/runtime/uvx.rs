use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

pub(super) fn check_wrapper(path: &Path, server: &str, version: &str) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", display_relative(path)))?;
    for required in [
        "command -v uvx",
        "CODEXY_UVX_PATH",
        "--no-config --isolated --default-index https://pypi.org/simple",
        &format!("codexy-runtime-tools=={version}"),
        &format!("codexy-mcp-runtime {server}"),
    ] {
        if !text.contains(required) {
            bail!(
                "{} must contain pinned uvx runtime contract {required:?}",
                display_relative(path)
            );
        }
    }
    for forbidden in [
        "python3",
        "cargo run",
        "cargo install",
        "curl ",
        "git clone",
        "dirname",
    ] {
        if text.contains(forbidden) {
            bail!(
                "{} must not contain runtime fallback {forbidden:?}",
                display_relative(path)
            );
        }
    }
    Ok(())
}
