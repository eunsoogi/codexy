use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const WRAPPERS: [(&str, &str); 2] = [
    ("plugins/codexy/mcp/codexy-mcp-lsp", "lsp"),
    ("plugins/codexy/mcp/codexy-mcp-codegraph", "codegraph"),
];
const PACKAGE_PREFIX: &str = "getcodexy==";

pub(super) fn check_version(expected: &str) -> Result<()> {
    check_version_at(&repo_root()?, expected)
}

pub(super) fn check_version_at(root: &Path, expected: &str) -> Result<()> {
    for (relative, server) in WRAPPERS {
        let path = root.join(relative);
        let actual = wrapper_pin(&path, server)?;
        if actual != expected {
            bail!(
                "version mismatch: {} {PACKAGE_PREFIX}{actual}, public bootstrap={expected}",
                display_relative(&path)
            );
        }
    }
    Ok(())
}

pub(super) fn prepare_pin_updates(root: &Path, version: &str) -> Result<Vec<WrapperUpdate>> {
    super::require_semver(version)?;
    WRAPPERS
        .iter()
        .map(|(relative, server)| {
            let path = root.join(relative);
            let (mut text, pin) = wrapper_pin_with_range(&path, server)?;
            text.replace_range(pin, version);
            Ok(WrapperUpdate {
                path,
                bytes: text.into_bytes(),
            })
        })
        .collect()
}

#[derive(Debug)]
pub(super) struct WrapperUpdate {
    pub(super) path: PathBuf,
    pub(super) bytes: Vec<u8>,
}

fn wrapper_pin(path: &PathBuf, server: &str) -> Result<String> {
    let (text, range) = wrapper_pin_with_range(path, server)?;
    Ok(text[range].to_owned())
}

fn wrapper_pin_with_range(path: &PathBuf, server: &str) -> Result<(String, Range<usize>)> {
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
    let runtime = format!(" codexy-mcp-runtime {server}");
    if !text[end..].starts_with(&runtime) {
        bail!(
            "{} pin must dispatch codexy-mcp-runtime {server}",
            display_relative(path)
        );
    }
    Ok((text, start..end))
}
