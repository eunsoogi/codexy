use std::{collections::BTreeSet, fs, path::Path};

use crate::paths::display_relative;

const CANONICAL_WRAPPER: &str = "plugins/codexy/hooks/codexy-authorized-squash-merge.sh";

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
            .any(|line| unguarded_merge(line))
        {
            errors.push(format!(
                "{} must validate authoritative merge authorization before mutation",
                display_relative(&path)
            ));
        }
    }
}

fn unguarded_merge(line: &str) -> bool {
    command_segments(line).into_iter().any(unguarded_segment)
}

fn unguarded_segment(segment: &str) -> bool {
    let tokens = segment.trim_start().split_whitespace().collect::<Vec<_>>();
    let tokens = strip_assignments(strip_condition(&tokens));
    let Some(tokens) = strip_env(tokens) else {
        return true;
    };
    let tokens = strip_assignments(tokens);
    tokens.first() != Some(&CANONICAL_WRAPPER) && tokens.starts_with(&["gh", "pr", "merge"])
}

fn command_segments(line: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quote != Some(b'\'') {
            escaped = true;
        } else if matches!(byte, b'\'' | b'\"') {
            quote = if quote == Some(byte) {
                None
            } else if quote.is_none() {
                Some(byte)
            } else {
                quote
            };
        } else if quote.is_none() && (byte == b';' || bytes.get(index..index + 2) == Some(b"&&")) {
            segments.push(&line[start..index]);
            index += usize::from(byte == b'&');
            start = index + 1;
        }
        index += 1;
    }
    segments.push(&line[start..]);
    segments
}

fn strip_condition<'a>(tokens: &'a [&'a str]) -> &'a [&'a str] {
    if tokens.starts_with(&["if", "!"]) {
        &tokens[2..]
    } else {
        tokens
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

fn command_blocks(text: &str) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut fence = None;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(marker) = fence_marker(line) {
            if fence == Some(marker) {
                blocks.push(std::mem::take(&mut current));
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            }
        } else if fence.is_some() && !line.starts_with('#') {
            current.push(line.to_owned());
        }
    }
    blocks
}

fn fence_marker(line: &str) -> Option<char> {
    line.starts_with("```")
        .then_some('`')
        .or_else(|| line.starts_with("~~~").then_some('~'))
}
