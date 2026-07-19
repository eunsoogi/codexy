use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

pub(super) fn check_wrapper(path: &Path, server: &str, version: &str) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", display_relative(path)))?;
    let active = active_shell(&text);
    for required in [
        "command -v uvx",
        "CODEXY_UVX_PATH",
        "--no-config --isolated --default-index https://pypi.org/simple",
        &format!("\"codexy-runtime-tools=={version}\""),
        &format!("codexy-mcp-runtime {server}"),
    ] {
        if !active.contains(required) {
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
        if active.contains(forbidden) {
            bail!(
                "{} must not contain runtime fallback {forbidden:?}",
                display_relative(path)
            );
        }
    }
    Ok(())
}

fn active_shell(text: &str) -> String {
    text.lines()
        .map(strip_shell_comment)
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_shell_comment(line: &str) -> &str {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    let mut token_boundary = true;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            token_boundary = false;
            continue;
        }
        if character == '\\' && !single_quoted {
            escaped = true;
            token_boundary = false;
            continue;
        }
        match character {
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            '#' if !single_quoted && !double_quoted && token_boundary => return &line[..index],
            _ => {}
        }
        token_boundary = !single_quoted
            && !double_quoted
            && (character.is_whitespace() || matches!(character, ';' | '|' | '&' | '(' | ')'));
    }
    line
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

    #[test]
    fn rejects_version_prefix_and_commented_execution_decoys() -> anyhow::Result<()> {
        for invalid in [
            VALID.replace("1.2.1", "1.2.10"),
            VALID
                .lines()
                .map(|line| {
                    if line.starts_with("exec ") || line.trim_start().starts_with("--from ") {
                        format!("# {line}")
                    } else {
                        line.to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        ] {
            let temp = tempfile::NamedTempFile::new()?;
            std::fs::write(temp.path(), invalid)?;
            assert!(check_wrapper(temp.path(), "lsp", "1.2.1").is_err());
        }
        Ok(())
    }

    #[test]
    fn ignores_forbidden_tokens_in_shell_comments() -> anyhow::Result<()> {
        let temp = tempfile::NamedTempFile::new()?;
        std::fs::write(
            temp.path(),
            format!("{VALID}\n# cargo install is intentionally forbidden\n"),
        )?;
        check_wrapper(temp.path(), "lsp", "1.2.1")
    }
}
