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
    let tokens = line.trim_start().split_whitespace().collect::<Vec<_>>();
    let tokens = strip_condition(&tokens);
    if tokens.first() == Some(&CANONICAL_WRAPPER) {
        return false;
    }
    let tokens = strip_env(tokens);
    tokens.starts_with(&["gh", "pr", "merge"])
}

fn strip_condition<'a>(tokens: &'a [&'a str]) -> &'a [&'a str] {
    if tokens.starts_with(&["if", "!"]) {
        &tokens[2..]
    } else {
        tokens
    }
}

fn strip_env<'a>(tokens: &'a [&'a str]) -> &'a [&'a str] {
    if tokens.first() == Some(&"env") {
        &tokens[1..]
    } else {
        tokens
    }
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
        } else if fence.is_some()
            && !line.starts_with('#')
            && !line.starts_with("echo")
            && !line.starts_with("printf")
        {
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
