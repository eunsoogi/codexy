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
    ">",
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
    "gh", "git", "mkdir", "touch", "rm", "mv", "cp", "chmod", "chown", "node", "nodejs",
];
const FORBIDDEN_COMMAND_TOKENS: &[&str] = &["node", "nodejs"];

pub(super) fn check_command_text(path: &Path, event: &str, command: &str) -> Result<()> {
    for forbidden in FORBIDDEN_COMMAND_FRAGMENTS {
        if command.contains(forbidden) {
            bail_forbidden_reference(path, event, forbidden)?;
        }
    }
    for forbidden in FORBIDDEN_COMMAND_TOKENS {
        if contains_shell_token(command, forbidden) {
            bail_forbidden_reference(path, event, forbidden)?;
        }
    }
    Ok(())
}

pub(super) fn check_script(path: &Path, event: &str, script_path: &Path) -> Result<()> {
    check_script_inner(path, event, script_path, FORBIDDEN_SCRIPT_FRAGMENTS, false)
}

pub(super) fn check_sourced_helper(path: &Path, event: &str, script_path: &Path) -> Result<()> {
    check_script_inner(
        path,
        event,
        script_path,
        FORBIDDEN_SOURCED_HELPER_FRAGMENTS,
        true,
    )
}

fn check_script_inner(
    path: &Path,
    event: &str,
    script_path: &Path,
    forbidden_fragments: &[&str],
    check_unquoted_redirection: bool,
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
    if check_unquoted_redirection && contains_sourced_helper_redirection(&text) {
        bail!(
            "{} {event} hook script must not contain shell redirection: {}",
            display_relative(path),
            display_relative(script_path)
        );
    }
    for line in text.lines().map(str::trim_start) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for command in FORBIDDEN_SCRIPT_COMMANDS {
            if contains_shell_token(line, command) {
                bail!(
                    "{} {event} hook script must not run {command:?}: {}",
                    display_relative(path),
                    display_relative(script_path)
                );
            }
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

fn is_shell_token_boundary(character: char) -> bool {
    !(character.is_ascii_alphanumeric() || character == '_' || character == '-')
}

fn contains_sourced_helper_redirection(text: &str) -> bool {
    let mut in_awk_program = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if in_awk_program {
            if trimmed == "'" {
                in_awk_program = false;
            }
            continue;
        }
        if trimmed.ends_with("awk '") || trimmed.contains(" awk '") {
            in_awk_program = true;
            continue;
        }
        if contains_unquoted_line_redirection(line) {
            return true;
        }
    }
    false
}

fn contains_unquoted_line_redirection(line: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    for character in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_double_quote && character == '\\' {
            escaped = true;
            continue;
        }
        match character {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '>' if !in_single_quote && !in_double_quote => return true,
            _ => {}
        }
    }
    false
}
