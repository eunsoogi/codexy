use std::path::Path;

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
    "npm",
    "curl",
    "codex plugin",
    "codex mcp",
];
const FORBIDDEN_SCRIPT_FRAGMENTS: &[&str] = &[
    "~/",
    "$HOME/",
    "$HOME",
    "${HOME}/",
    "${HOME}",
    "/Users/",
    "/home/",
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
    "npm",
    "curl",
    "codex plugin",
    "codex mcp",
    ">",
];
const FORBIDDEN_SOURCED_HELPER_FRAGMENTS: &[&str] = &[
    "~/",
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
    "npm",
    "curl",
    "codex plugin",
    "codex mcp",
];
const FORBIDDEN_SCRIPT_COMMANDS: &[&str] = &[
    "gh", "git", "mkdir", "touch", "rm", "mv", "cp", "chmod", "chown", "node",
];

pub(super) fn check_command_text(path: &Path, event: &str, command: &str) -> Result<()> {
    for forbidden in FORBIDDEN_COMMAND_FRAGMENTS {
        if command.contains(forbidden) {
            bail_forbidden_reference(path, event, forbidden)?;
        }
    }
    if contains_shell_token(command, "node") {
        bail_forbidden_reference(path, event, "node")?;
    }
    Ok(())
}

pub(super) fn check_script(path: &Path, event: &str, script_path: &Path) -> Result<()> {
    check_script_inner(path, event, script_path, FORBIDDEN_SCRIPT_FRAGMENTS)
}

pub(super) fn check_sourced_helper(path: &Path, event: &str, script_path: &Path) -> Result<()> {
    check_script_inner(path, event, script_path, FORBIDDEN_SOURCED_HELPER_FRAGMENTS)
}

fn check_script_inner(
    path: &Path,
    event: &str,
    script_path: &Path,
    forbidden_fragments: &[&str],
) -> Result<()> {
    let text = std::fs::read_to_string(script_path)
        .with_context(|| format!("reading {}", display_relative(script_path)))?;
    for forbidden in forbidden_fragments {
        if text.contains(forbidden) {
            bail!(
                "{} {event} hook script must not contain {forbidden:?}: {}",
                display_relative(path),
                display_relative(script_path)
            );
        }
    }
    for line in text.lines().map(str::trim_start) {
        let Some(command) = first_shell_token(line) else {
            continue;
        };
        if FORBIDDEN_SCRIPT_COMMANDS.contains(&command) {
            bail!(
                "{} {event} hook script must not run {command:?}: {}",
                display_relative(path),
                display_relative(script_path)
            );
        }
    }
    Ok(())
}

fn bail_forbidden_reference(path: &Path, event: &str, forbidden: &str) -> Result<()> {
    bail!(
        "{} {event} hook command must not reference {forbidden:?}",
        display_relative(path)
    );
}

fn contains_shell_token(text: &str, token: &str) -> bool {
    text.split(is_shell_token_boundary)
        .any(|part| part == token)
}

fn first_shell_token(line: &str) -> Option<&str> {
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    line.split(is_shell_token_boundary)
        .find(|part| !part.is_empty())
}

fn is_shell_token_boundary(character: char) -> bool {
    !(character.is_ascii_alphanumeric() || character == '_' || character == '-')
}
