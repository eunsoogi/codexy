use std::{fs, ops::Range, path::Path};

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const WRAPPER: &str = "plugins/codexy-devtools/mcp/codexy-mcp-devtools";
const PACKAGE_PREFIX: &str = "getcodexy==";

pub(super) fn check_version(expected: &str) -> Result<()> {
    check_version_at(&repo_root()?, expected)
}

pub(super) fn check_version_at(root: &Path, expected: &str) -> Result<()> {
    let path = root.join(WRAPPER);
    let path = if path.exists() {
        path
    } else {
        root.join("plugins/codexy-devtools/mcp/codexy-mcp-lsp")
    };
    let actual = wrapper_pin(&path)?;
    if actual != expected {
        bail!(
            "version mismatch: {} {PACKAGE_PREFIX}{actual}, public bootstrap={expected}",
            display_relative(&path)
        );
    }
    Ok(())
}

fn wrapper_pin(path: &Path) -> Result<String> {
    let (text, range) = wrapper_pin_with_range(path)?;
    Ok(text[range].to_owned())
}

fn wrapper_pin_with_range(path: &Path) -> Result<(String, Range<usize>)> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let dispatch = format!("exec uvx --from {PACKAGE_PREFIX}");
    let matches = text.match_indices(&dispatch).collect::<Vec<_>>();
    if matches.len() != 1 || text.match_indices(PACKAGE_PREFIX).count() != 1 {
        bail!(
            "{} must contain exactly one executable {PACKAGE_PREFIX}VERSION pin",
            display_relative(path)
        );
    }
    let start = matches[0].0 + dispatch.len();
    let end = text[start..]
        .find(char::is_whitespace)
        .map_or(text.len(), |offset| start + offset);
    let version = &text[start..end];
    super::require_semver(version)?;
    if !text[end..].starts_with(" codexy-mcp-runtime \"$server\"")
        && !text[end..].starts_with(" codexy-mcp-runtime lsp")
    {
        bail!(
            "{} pin must dispatch the selected codexy-mcp-runtime server",
            display_relative(path)
        );
    }
    Ok((text, start..end))
}
