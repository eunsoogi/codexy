use std::{collections::BTreeSet, fs, path::Path};

use crate::paths::display_relative;

const CANONICAL_WRAPPER: &str = "plugins/codexy/hooks/codexy-authorized-squash-merge.sh";
const MAX_SHELL_NESTING: usize = 8;

#[path = "merge_authorization_routes/segments.rs"]
mod segments;
#[path = "shell_options.rs"]
mod shell_options;

use segments::{command_blocks, command_segments};

pub(super) fn check(root: &Path, errors: &mut Vec<String>) {
    let mut paths = BTreeSet::new();
    if super::collect_markdown(&root.join("skills"), &mut paths).is_err() {
        errors.push("skills could not be read for merge-authorization policy".into());
        return;
    }
    for path in paths {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if command_blocks(&text)
            .iter()
            .flatten()
            .any(|line| unguarded_merge_at(line, 0))
        {
            errors.push(format!(
                "{} must validate authoritative merge authorization before mutation",
                display_relative(&path)
            ));
        }
    }
}

fn unguarded_merge_at(line: &str, depth: usize) -> bool {
    command_segments(line)
        .into_iter()
        .any(|segment| unguarded_segment(segment, depth))
}

fn unguarded_segment(segment: &str, depth: usize) -> bool {
    let words = shell_words(segment);
    let token_refs = words.iter().map(String::as_str).collect::<Vec<_>>();
    let tokens = executable_tokens(&token_refs);
    let Some(tokens) = tokens else {
        return true;
    };
    match shell_options::invocation(tokens) {
        shell_options::Invocation::Command(program) => {
            return depth >= MAX_SHELL_NESTING || unguarded_merge_at(program, depth + 1);
        }
        shell_options::Invocation::Invalid => return true,
        shell_options::Invocation::NotShell | shell_options::Invocation::Safe => {}
    }
    tokens.first() != Some(&CANONICAL_WRAPPER) && tokens.starts_with(&["gh", "pr", "merge"])
}

fn executable_tokens<'a>(tokens: &'a [&'a str]) -> Option<&'a [&'a str]> {
    let tokens = strip_assignments(strip_controls(&tokens));
    let Some(tokens) = strip_command(tokens) else {
        return None;
    };
    let Some(tokens) = strip_env(tokens) else {
        return None;
    };
    let tokens = strip_assignments(tokens);
    strip_command(tokens)
}

fn shell_words(segment: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in segment.chars() {
        if escaped {
            word.push(character);
            escaped = false;
        } else if character == '\\' && quote != Some('\'') {
            escaped = true;
        } else if matches!(character, '\'' | '\"') {
            quote = if quote == Some(character) {
                None
            } else if quote.is_none() {
                Some(character)
            } else {
                quote
            };
        } else if quote.is_none() && character.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(character);
        }
    }
    if escaped {
        word.push('\\');
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

fn strip_controls<'a>(mut tokens: &'a [&'a str]) -> &'a [&'a str] {
    while matches!(
        tokens.first(),
        Some(&"if" | &"then" | &"while" | &"until" | &"do" | &"!" | &"{" | &"}")
    ) {
        tokens = &tokens[1..];
    }
    tokens
}

fn strip_command<'a>(tokens: &'a [&'a str]) -> Option<&'a [&'a str]> {
    if tokens.first() != Some(&"command") {
        return Some(tokens);
    }
    match &tokens[1..] {
        ["-v" | "-V", ..] => Some(&[]),
        ["--", rest @ ..] | ["-p", rest @ ..] => Some(rest),
        [option, ..] if option.starts_with('-') => None,
        rest => Some(rest),
    }
}

fn strip_env<'a>(mut tokens: &'a [&'a str]) -> Option<&'a [&'a str]> {
    if tokens.first() != Some(&"env") {
        return Some(tokens);
    }
    tokens = &tokens[1..];
    while let Some(token) = tokens.first() {
        match *token {
            "--" => return Some(&tokens[1..]),
            "-i" | "--ignore-environment" => tokens = &tokens[1..],
            "-u" | "--unset" => tokens = tokens.get(2..)?,
            _ if assignment(token) || token.starts_with("-u") || token.starts_with("--unset=") => {
                tokens = &tokens[1..]
            }
            _ if token.starts_with('-') => return None,
            _ => return Some(tokens),
        }
    }
    Some(tokens)
}

fn strip_assignments<'a>(mut tokens: &'a [&'a str]) -> &'a [&'a str] {
    while tokens.first().is_some_and(|token| assignment(token)) {
        tokens = &tokens[1..];
    }
    tokens
}

fn assignment(token: &str) -> bool {
    token.split_once('=').is_some_and(|(name, _)| {
        name.starts_with(|character: char| character.is_ascii_alphabetic() || character == '_')
            && name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
    })
}
