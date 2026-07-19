use std::fs;

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const PACKAGE_NAME: &str = "codexy-runtime-tools";

fn pyproject() -> Result<std::path::PathBuf> {
    Ok(repo_root()?.join("packages/codexy-runtime-tools/pyproject.toml"))
}

fn wrappers() -> Result<Vec<std::path::PathBuf>> {
    let root = repo_root()?.join("plugins/codexy/mcp");
    Ok(["lsp", "codegraph"]
        .into_iter()
        .map(|server| root.join(format!("codexy-mcp-{server}")))
        .collect())
}

fn package_version(text: &str) -> Option<&str> {
    text.lines()
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
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
    for wrapper in wrappers()? {
        let wrapper_text = fs::read_to_string(&wrapper)
            .with_context(|| format!("missing required file: {}", display_relative(&wrapper)))?;
        if !wrapper_text.contains(&expected_pin) {
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
    let current_pin = format!("{PACKAGE_NAME}=={current}");
    let requested_pin = format!("{PACKAGE_NAME}=={requested}");
    for wrapper in wrappers()? {
        let text = fs::read_to_string(&wrapper)?;
        if text.matches(&current_pin).count() != 1 {
            bail!(
                "{} must contain exactly one {current_pin}",
                display_relative(&wrapper)
            );
        }
        fs::write(&wrapper, text.replace(&current_pin, &requested_pin))?;
    }
    Ok(())
}
