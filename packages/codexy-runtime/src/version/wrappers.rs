use std::{fs, ops::Range, path::Path};

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const WRAPPER: &str = "plugins/codexy-devtools/mcp/codexy-mcp-devtools";
const LEGACY_WRAPPER: &str = "plugins/codexy-devtools/mcp/codexy-mcp-lsp";
const CORE_WRAPPERS: [&str; 2] = [
    "plugins/codexy/mcp/codexy-mcp-watcher.sh",
    "plugins/codexy/mcp/codexy-mcp-watcher.cmd",
];
const PACKAGE_PREFIX: &str = "getcodexy==";

pub(super) fn check_version(expected: &str) -> Result<()> {
    check_version_at(&repo_root()?, expected)
}

pub(super) fn check_version_at(root: &Path, expected: &str) -> Result<()> {
    super::mcp_config::check_at(root)?;
    let path = root.join(WRAPPER);
    let path = if path.exists() {
        path
    } else {
        root.join(LEGACY_WRAPPER)
    };
    check_wrapper_version(&path, expected)?;
    let core_present = CORE_WRAPPERS
        .iter()
        .any(|relative| root.join(relative).exists())
        || root.join("plugins/codexy/.mcp.json").exists();
    if core_present {
        for relative in CORE_WRAPPERS {
            check_wrapper_version(&root.join(relative), expected)?;
        }
    }
    Ok(())
}

fn check_wrapper_version(path: &Path, expected: &str) -> Result<()> {
    let actual = wrapper_pin(path)?;
    if actual == expected {
        return Ok(());
    }
    bail!(
        "version mismatch: {} {PACKAGE_PREFIX}{actual}, public bootstrap={expected}",
        display_relative(path)
    )
}

fn wrapper_pin(path: &Path) -> Result<String> {
    let (text, range) = wrapper_pin_with_range(path)?;
    Ok(text[range].to_owned())
}

fn wrapper_pin_with_range(path: &Path) -> Result<(String, Range<usize>)> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let matches = text.match_indices(PACKAGE_PREFIX).collect::<Vec<_>>();
    if matches.len() != 1 {
        bail!(
            "{} must contain exactly one executable {PACKAGE_PREFIX}VERSION pin",
            display_relative(path)
        );
    }
    let prefix_end = matches[0].0;
    if !text[..prefix_end].ends_with("exec uvx --from ")
        && !text[..prefix_end].ends_with("uvx --from ")
    {
        bail!(
            "{} must contain exactly one executable {PACKAGE_PREFIX}VERSION pin",
            display_relative(path)
        );
    }
    let start = prefix_end + PACKAGE_PREFIX.len();
    let end = text[start..]
        .find(char::is_whitespace)
        .map_or(text.len(), |offset| start + offset);
    let version = &text[start..end];
    super::require_semver(version)?;
    if !text[end..].starts_with(" codexy-mcp-runtime \"$server\"")
        && !text[end..].starts_with(" codexy-mcp-runtime lsp")
        && !text[end..].starts_with(" codexy-mcp-runtime watcher")
    {
        bail!(
            "{} pin must dispatch the selected codexy-mcp-runtime server",
            display_relative(path)
        );
    }
    Ok((text, start..end))
}
