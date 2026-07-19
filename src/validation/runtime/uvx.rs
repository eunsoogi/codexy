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

#[cfg(test)]
mod tests {
    use super::check_wrapper;

    const VALID: &str = r#"#!/bin/sh
bundled_platforms="linux-x86_64 darwin-arm64"
command -v uvx
CODEXY_UVX_PATH=uvx
exec uvx --no-config --isolated --default-index https://pypi.org/simple \
  --from "codexy-runtime-tools==1.2.1" codexy-mcp-runtime lsp
"#;

    #[test]
    fn rejects_missing_required_uvx_contract() -> anyhow::Result<()> {
        for required in [
            "command -v uvx",
            "CODEXY_UVX_PATH",
            "--no-config --isolated --default-index https://pypi.org/simple",
            "codexy-runtime-tools==1.2.1",
            "codexy-mcp-runtime lsp",
        ] {
            let temp = tempfile::NamedTempFile::new()?;
            std::fs::write(temp.path(), VALID.replacen(required, "", 1))?;
            assert!(check_wrapper(temp.path(), "lsp", "1.2.1").is_err());
        }
        Ok(())
    }

    #[test]
    fn rejects_forbidden_runtime_fallback() -> anyhow::Result<()> {
        for forbidden in [
            "python3",
            "cargo run",
            "cargo install",
            "curl ",
            "git clone",
            "dirname",
        ] {
            let temp = tempfile::NamedTempFile::new()?;
            std::fs::write(temp.path(), format!("{VALID}\n{forbidden}\n"))?;
            assert!(check_wrapper(temp.path(), "lsp", "1.2.1").is_err());
        }
        Ok(())
    }
}
