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
    if has_single_quoted_plugin_root_entrypoint(command) {
        bail!(
            "{} {event} hook command single-quoted PLUGIN_ROOT entrypoints are not supported",
            display_relative(path)
        );
    }
    if has_quoted_entrypoint_without_separator(command) {
        bail!(
            "{} {event} hook command quoted hook entrypoints must be followed by whitespace or EOF",
            display_relative(path)
        );
    }
    let (hook_path, arguments) = plugin_root_entrypoint_path(command).with_context(|| {
        format!(
            "{} {event} hook command must start with a packaged ${{PLUGIN_ROOT}} entrypoint",
            display_relative(path)
        )
    })?;
    if !command.trim_start().starts_with('"') && hook_path_has_shell_control_syntax(&hook_path) {
        bail!(
            "{} {event} hook command unquoted hook entrypoints must not contain shell control syntax",
            display_relative(path)
        );
    }
    check_static_arguments(path, event, arguments)?;
    let canonical_root = plugin_root.canonicalize().with_context(|| {
        format!(
            "{} plugin root cannot be canonicalized",
            display_relative(plugin_root)
        )
    })?;
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
    Ok(())
}

fn plugin_root_entrypoint_path(command: &str) -> Option<(PathBuf, &str)> {
    let entrypoint = command_entrypoint(command)?;
    for marker in ["${PLUGIN_ROOT}/", "$PLUGIN_ROOT/"] {
        if let Some(path) = entrypoint.command.strip_prefix(marker) {
            return (!path.is_empty()).then(|| (PathBuf::from(path), entrypoint.arguments));
        }
    }
    None
}

fn has_single_quoted_plugin_root_entrypoint(command: &str) -> bool {
    let command = command.trim_start();
    if !command.starts_with('\'') {
        return false;
    }
    let Some(close) = command[1..].find('\'') else {
        return false;
    };
    let quoted = &command[1..=close];
    ["${PLUGIN_ROOT}/", "$PLUGIN_ROOT/"]
        .iter()
        .any(|marker| quoted.starts_with(marker))
}

fn has_quoted_entrypoint_without_separator(command: &str) -> bool {
    let command = command.trim_start();
    if !command.starts_with('"') {
        return false;
    }
    let Some(close) = command[1..].find('"') else {
        return false;
    };
    let after_quote = &command[close + 2..];
    !after_quote.is_empty() && !after_quote.starts_with(char::is_whitespace)
}

struct CommandEntrypoint<'a> {
    command: String,
    arguments: &'a str,
}

fn command_entrypoint(command: &str) -> Option<CommandEntrypoint<'_>> {
    let command = command.trim_start();
    let first = command.chars().next()?;
    if first == '"' || first == '\'' {
        if first == '\'' {
            return None;
        }
        let close = command[1..].find(first)?;
        if close == 0 {
            return Some(CommandEntrypoint {
                command: String::new(),
                arguments: &command[2..],
            });
        }
        return Some(CommandEntrypoint {
            command: command[1..=close].to_string(),
            arguments: &command[close + 2..],
        });
    }
    for (index, character) in command.char_indices() {
        if character.is_whitespace() {
            return Some(CommandEntrypoint {
                command: command[..index].to_string(),
                arguments: &command[index..],
            });
        }
    }
    Some(CommandEntrypoint {
        command: command.to_string(),
        arguments: "",
    })
}

fn check_static_arguments(path: &Path, event: &str, arguments: &str) -> Result<()> {
    if arguments.chars().all(is_static_argument_character) {
        return Ok(());
    }
    bail!(
        "{} {event} hook command arguments must be static values without shell control syntax",
        display_relative(path)
    );
}

fn hook_path_has_shell_control_syntax(path: &Path) -> bool {
    path.to_string_lossy()
        .chars()
        .any(|character| matches!(character, ';' | '&' | '|' | '<' | '>' | '(' | ')' | '`'))
}

fn is_static_argument_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || character == ' '
        || character == '\t'
        || matches!(character, '-' | '_' | '.' | '/' | ':')
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
