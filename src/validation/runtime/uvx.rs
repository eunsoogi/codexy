use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;
use crate::shell::{commands, has_sequence, runtime_exec, unique_option_value};

pub(super) fn check_wrapper(path: &Path, server: &str, version: &str) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", display_relative(path)))?;
    let commands = commands(&text);
    let has_uvx_override = commands
        .iter()
        .flatten()
        .any(|word| matches!(word.text.as_str(), "$CODEXY_UVX_PATH") || word.text.starts_with("CODEXY_UVX_PATH="));
    if !commands
        .iter()
        .any(|command| has_sequence(command, &["command", "-v", "uvx"]))
        || !has_uvx_override
    {
        bail!("{} must locate uvx safely", display_relative(path));
    }
    let command = runtime_exec(&text, server).with_context(|| {
        format!(
            "{} must contain exactly one active runtime exec command",
            display_relative(path)
        )
    })?;
    if !command
        .get(1)
        .is_some_and(|word| matches!(word.text.as_str(), "uvx" | "$uvx_path"))
        || unique_option_value(&command, "--from")
            != Some(format!("eunsoogi-codexy=={version}").as_str())
    {
        bail!(
            "{} has an invalid runtime executable or pin",
            display_relative(path)
        );
    }
    for required in [
        vec!["--no-config"],
        vec!["--isolated"],
        vec!["--default-index", "https://pypi.org/simple"],
        vec!["codexy-mcp-runtime", server],
    ] {
        if !has_sequence(&command, &required) {
            bail!(
                "{} has an invalid runtime exec command",
                display_relative(path)
            );
        }
    }
    for forbidden in [
        &["python3"][..],
        &["cargo", "run"],
        &["cargo", "install"],
        &["curl"],
        &["git", "clone"],
        &["dirname"],
    ] {
        if commands
            .iter()
            .any(|command| has_sequence(command, forbidden))
        {
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
  --from "eunsoogi-codexy==1.2.1" codexy-mcp-runtime lsp
"#;

    #[test]
    fn rejects_missing_required_uvx_contract() -> anyhow::Result<()> {
        for required in [
            "command -v uvx",
            "CODEXY_UVX_PATH",
            "--no-config --isolated --default-index https://pypi.org/simple",
            "eunsoogi-codexy==1.2.1",
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
            VALID.replace("1.2.1", "1.2\\.1"),
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
    fn rejects_duplicate_from_and_non_uvx_exec_commands() -> anyhow::Result<()> {
        for invalid in [
            VALID.replace(
                "--from \"eunsoogi-codexy==1.2.1\"",
                "--from \"eunsoogi-codexy==1.2.1\" --from \"eunsoogi-codexy==1.2.10\"",
            ),
            VALID.replace("exec uvx", "exec arbitrary-runner"),
        ] {
            let temp = tempfile::NamedTempFile::new()?;
            std::fs::write(temp.path(), invalid)?;
            assert!(check_wrapper(temp.path(), "lsp", "1.2.1").is_err());
        }
        Ok(())
    }

    #[test]
    fn rejects_substring_only_uvx_override_variables() -> anyhow::Result<()> {
        let temp = tempfile::NamedTempFile::new()?;
        std::fs::write(temp.path(), VALID.replace("CODEXY_UVX_PATH", "NOT_CODEXY_UVX_PATH"))?;
        assert!(check_wrapper(temp.path(), "lsp", "1.2.1").is_err());
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
