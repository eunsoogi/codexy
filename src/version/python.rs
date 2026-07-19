use std::fs;

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};
use crate::shell::{replace_runtime_pin, runtime_exec, unique_option_value};

const PACKAGE_NAME: &str = "eunsoogi-codexy";

fn pyproject() -> Result<std::path::PathBuf> {
    Ok(repo_root()?.join("packages/eunsoogi-codexy/pyproject.toml"))
}

fn wrappers() -> Result<Vec<(&'static str, std::path::PathBuf)>> {
    let root = repo_root()?.join("plugins/codexy/mcp");
    Ok(["lsp", "codegraph"]
        .into_iter()
        .map(|server| (server, root.join(format!("codexy-mcp-{server}"))))
        .collect())
}

fn package_version(text: &str) -> Option<&str> {
    text.lines()
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
}

fn wrapper_has_pin(text: &str, server: &str, expected_pin: &str) -> bool {
    runtime_exec(text, server)
        .is_some_and(|command| unique_option_value(&command, "--from") == Some(expected_pin))
}

pub(super) fn check_version(expected: &str) -> Result<()> {
    let path = pyproject()?;
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    let observed = package_version(&text)
        .with_context(|| format!("{} must declare project version", display_relative(&path)))?;
    if observed != expected {
        bail!(
            "version mismatch: {}={observed}, plugin manifest={expected}",
            display_relative(&path)
        );
    }
    let expected_pin = format!("{PACKAGE_NAME}=={expected}");
    for (server, wrapper) in wrappers()? {
        let wrapper_text = fs::read_to_string(&wrapper)
            .with_context(|| format!("missing required file: {}", display_relative(&wrapper)))?;
        if !wrapper_has_pin(&wrapper_text, server, &expected_pin) {
            bail!(
                "version mismatch: {} must pin {expected_pin}",
                display_relative(&wrapper)
            );
        }
    }
    Ok(())
}

pub(super) fn set_version(current: &str, requested: &str) -> Result<()> {
    let path = pyproject()?;
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    let current_line = format!("version = \"{current}\"");
    if text.matches(&current_line).count() != 1 {
        bail!(
            "{} must contain exactly one {current_line:?}",
            display_relative(&path)
        );
    }
    fs::write(
        &path,
        text.replace(&current_line, &format!("version = \"{requested}\"")),
    )?;
    let current_pin = format!("\"{PACKAGE_NAME}=={current}\"");
    let requested_pin = format!("\"{PACKAGE_NAME}=={requested}\"");
    for (server, wrapper) in wrappers()? {
        let text = fs::read_to_string(&wrapper)?;
        let updated = replace_runtime_pin(
            &text,
            server,
            current_pin.trim_matches('"'),
            requested_pin.trim_matches('"'),
        )
        .with_context(|| {
            format!(
                "{} must execute exactly one runtime command pinned to {current_pin}",
                display_relative(&wrapper)
            )
        })?;
        fs::write(&wrapper, updated)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::wrapper_has_pin;

    #[test]
    fn wrapper_pin_requires_exact_active_shell_token() {
        assert!(wrapper_has_pin(
            "exec uvx --from \"eunsoogi-codexy==1.2.1\" codexy-mcp-runtime lsp",
            "lsp",
            "eunsoogi-codexy==1.2.1"
        ));
        assert!(!wrapper_has_pin(
            "exec uvx --from \"eunsoogi-codexy==1.2.10\" codexy-mcp-runtime lsp",
            "lsp",
            "eunsoogi-codexy==1.2.1"
        ));
        assert!(!wrapper_has_pin(
            "# exec uvx --from \"eunsoogi-codexy==1.2.1\" codexy-mcp-runtime lsp",
            "lsp",
            "eunsoogi-codexy==1.2.1"
        ));
    }
}
