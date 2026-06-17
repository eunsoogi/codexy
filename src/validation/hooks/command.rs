use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

const FORBIDDEN_COMMAND_FRAGMENTS: &[&str] = &[
    "~",
    "$HOME",
    "${HOME}",
    "/Users/",
    "/home/",
    ".codex/",
    ".git/",
    "auth.json",
    "history.jsonl",
    "PLUGIN_DATA",
    "CLAUDE_PLUGIN_DATA",
    "python",
    "node",
    "npm",
    "curl",
    "codex plugin",
    "codex mcp",
];
const FORBIDDEN_SCRIPT_FRAGMENTS: &[&str] = &[
    "~/.codex",
    "$HOME/.codex",
    "${HOME}/.codex",
    ".codex/",
    ".git/",
    "auth.json",
    "history.jsonl",
    "PLUGIN_DATA",
    "CLAUDE_PLUGIN_DATA",
    "python",
    "node",
    "npm",
    "curl",
    "codex plugin",
    "codex mcp",
    ">>",
];
const FORBIDDEN_SCRIPT_COMMAND_PREFIXES: &[&str] = &[
    "gh ", "git ", "mkdir ", "touch ", "rm ", "mv ", "cp ", "chmod ", "chown ",
];

pub(super) fn check_command(
    path: &Path,
    plugin_root: &Path,
    event: &str,
    command: &str,
) -> Result<()> {
    for forbidden in FORBIDDEN_COMMAND_FRAGMENTS {
        if command.contains(forbidden) {
            bail!(
                "{} {event} hook command must not reference {forbidden:?}",
                display_relative(path)
            );
        }
    }
    let hook_paths = plugin_root_command_paths(command);
    if hook_paths.is_empty() {
        bail!(
            "{} {event} hook command must reference a packaged ${{PLUGIN_ROOT}} path",
            display_relative(path)
        );
    }
    let canonical_root = plugin_root.canonicalize().with_context(|| {
        format!(
            "{} plugin root cannot be canonicalized",
            display_relative(plugin_root)
        )
    })?;
    for hook_path in hook_paths {
        let resolved = plugin_root.join(&hook_path);
        if !resolved.is_file() {
            bail!(
                "{} {event} hook command target does not exist: {}",
                display_relative(path),
                display_relative(&resolved)
            );
        }
        let canonical_resolved = resolved.canonicalize().with_context(|| {
            format!(
                "{} {event} hook command target cannot be canonicalized: {}",
                display_relative(path),
                display_relative(&resolved)
            )
        })?;
        if !canonical_resolved.starts_with(&canonical_root) {
            bail!(
                "{} {event} hook command target must stay inside the plugin root",
                display_relative(path)
            );
        }
        check_script_safety(path, event, &resolved)?;
    }
    Ok(())
}

fn plugin_root_command_paths(command: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for marker in ["${PLUGIN_ROOT}/", "$PLUGIN_ROOT/"] {
        let mut rest = command;
        while let Some((_, after_marker)) = rest.split_once(marker) {
            let path = after_marker
                .split(|character: char| {
                    character.is_whitespace() || character == '"' || character == '\''
                })
                .next()
                .unwrap_or_default();
            if !path.is_empty() {
                paths.push(PathBuf::from(path));
            }
            rest = after_marker;
        }
    }
    paths
}

fn check_script_safety(path: &Path, event: &str, script_path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(script_path)
        .with_context(|| format!("reading {}", display_relative(script_path)))?;
    for forbidden in FORBIDDEN_SCRIPT_FRAGMENTS {
        if text.contains(forbidden) {
            bail!(
                "{} {event} hook script must not contain {forbidden:?}: {}",
                display_relative(path),
                display_relative(script_path)
            );
        }
    }
    for line in text.lines().map(str::trim_start) {
        for forbidden in FORBIDDEN_SCRIPT_COMMAND_PREFIXES {
            if line.starts_with(forbidden) {
                bail!(
                    "{} {event} hook script must not run {forbidden:?}: {}",
                    display_relative(path),
                    display_relative(script_path)
                );
            }
        }
    }
    Ok(())
}
