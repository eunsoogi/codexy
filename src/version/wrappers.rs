use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result, bail};

use crate::paths::{display_relative, repo_root};

const WRAPPERS: [(&str, &str); 2] = [
    ("plugins/codexy/mcp/codexy-mcp-lsp", "lsp"),
    ("plugins/codexy/mcp/codexy-mcp-codegraph", "codegraph"),
];
const PACKAGE_PREFIX: &str = "getcodexy==";

pub(super) fn check_version(expected: &str) -> Result<()> {
    for (path, server) in wrapper_paths()? {
        let (text, range) = wrapper_pin(&path, server)?;
        let actual = &text[range];
        if actual != expected {
            bail!(
                "version mismatch: {} {PACKAGE_PREFIX}{actual}, plugin manifest={expected}",
                display_relative(&path)
            );
        }
    }
    Ok(())
}

pub(super) fn set_version(version: &str) -> Result<()> {
    let updates = wrapper_paths()?
        .into_iter()
        .map(|(path, server)| {
            let (mut text, range) = wrapper_pin(&path, server)?;
            text.replace_range(range, version);
            Ok((path, text))
        })
        .collect::<Result<Vec<_>>>()?;
    for (path, text) in updates {
        fs::write(&path, text).with_context(|| format!("writing {}", display_relative(&path)))?;
    }
    Ok(())
}

fn wrapper_paths() -> Result<Vec<(PathBuf, &'static str)>> {
    let root = repo_root()?;
    Ok(WRAPPERS
        .iter()
        .map(|(path, server)| (root.join(path), *server))
        .collect())
}

fn wrapper_pin(path: &PathBuf, server: &str) -> Result<(String, std::ops::Range<usize>)> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let dispatch = format!("exec uvx --from {PACKAGE_PREFIX}");
    let matches = text.match_indices(&dispatch).collect::<Vec<_>>();
    if matches.len() != 1 || text.match_indices(PACKAGE_PREFIX).count() != 1 {
        bail!(
            "{} must contain exactly one executable {PACKAGE_PREFIX}VERSION pin",
            display_relative(path),
        );
    }
    let command_start = matches[0].0;
    let line_start = text[..command_start]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    if !text[line_start..command_start].trim().is_empty() {
        bail!(
            "{} pin must be an executable dispatch",
            display_relative(path)
        );
    }
    let start = command_start + dispatch.len();
    let end = text[start..]
        .find(char::is_whitespace)
        .map_or(text.len(), |offset| start + offset);
    let version = &text[start..end];
    super::require_semver(version).with_context(|| {
        format!(
            "{} {PACKAGE_PREFIX} pin must be MAJOR.MINOR.PATCH",
            display_relative(path)
        )
    })?;
    let runtime = format!(" codexy-mcp-runtime {server}");
    let suffix = &text[end..];
    if !suffix.starts_with(&runtime) || !suffix[runtime.len()..].starts_with(char::is_whitespace) {
        bail!(
            "{} pin must dispatch codexy-mcp-runtime {server}",
            display_relative(path)
        );
    }
    Ok((text, start..end))
}
