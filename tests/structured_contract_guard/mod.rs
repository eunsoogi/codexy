#![allow(dead_code)]

mod identity;
mod sanitize;
mod repository;

use identity::assertion_identity;
use sanitize::{sanitize, strip_comments};
pub(crate) use repository::{
    comparison_counts, comparison_counts_at, repository_violations, repository_violations_at,
};

const RATIONALE: &str = "structured-contract: non-contract substring rationale:";

pub(crate) struct GovernedAssertion {
    pub(crate) diagnostic: String,
    pub(crate) identity: String,
}

pub(crate) fn scan_source(source: &str) -> Vec<String> {
    governed_assertions(source)
        .into_iter()
        .map(|assertion| assertion.diagnostic)
        .collect()
}

pub(crate) fn governed_assertions(source: &str) -> Vec<GovernedAssertion> {
    let clean = sanitize(source);
    let provenance = strip_comments(source);
    let governed = governed_bindings(&provenance);
    assertions(&clean)
        .into_iter()
        .filter_map(|(start, end, body)| {
            if !has_contains_call(body) {
                return None;
            }
            let identifiers = identifiers(body);
            let governed_receiver = identifiers
                .iter()
                .find(|name| governed.iter().any(|bound| bound == **name));
            if let Some(receiver) = governed_receiver {
                return Some(GovernedAssertion {
                    diagnostic: format!("line {} receiver `{receiver}`", line_number(source, start)),
                    identity: assertion_identity(&provenance[start..=end]),
                });
            }
            if identifiers.iter().any(|name| is_diagnostic(name)) || has_rationale(source, start) {
                return None;
            }
            let receiver = identifiers.first()?;
            Some(GovernedAssertion {
                diagnostic: format!("line {} receiver `{receiver}`", line_number(source, start)),
                identity: assertion_identity(&provenance[start..=end]),
            })
        })
        .collect()
}

fn has_contains_call(text: &str) -> bool {
    let mut tail = text;
    while let Some(index) = tail.find(".contains") {
        let after = &tail[index + ".contains".len()..];
        if after.trim_start().starts_with('(') {
            return true;
        }
        tail = after;
    }
    false
}

fn governed_bindings(source: &str) -> Vec<String> {
    let mut governed: Vec<String> = Vec::new();
    let mut governed_paths: Vec<String> = Vec::new();
    for statement in source.split(';') {
        let Some(name) = let_binding(statement) else {
            continue;
        };
        let reads_document = statement.contains("read_to_string");
        let governed_path = is_governed_path(statement)
            || governed_paths
                .iter()
                .any(|bound| contains_identifier(statement, bound));
        let alias = governed
            .iter()
            .any(|bound| contains_identifier(statement, bound));
        if reads_document && governed_path || alias {
            governed.push(name.to_owned());
        } else if governed_path {
            governed_paths.push(name.to_owned());
        }
    }
    governed
}

fn let_binding(statement: &str) -> Option<&str> {
    let (_, tail) = statement.rsplit_once("let ")?;
    let name = tail
        .trim_start()
        .trim_start_matches("mut ")
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .next()?;
    (!name.is_empty()).then_some(name)
}

fn is_governed_path(text: &str) -> bool {
    [
        "plugins/codexy/skills/",
        "plugins/codexy/agents/",
        "AGENTS.md",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn assertions(source: &str) -> Vec<(usize, usize, &str)> {
    let mut found = Vec::new();
    let mut offset = 0;
    while let Some(relative) = source[offset..].find("assert") {
        let start = offset + relative;
        let Some(open) = assertion_open(source, start) else {
            offset = start + "assert".len();
            continue;
        };
        if let Some(close) = matching_paren(source, open) {
            found.push((start, close, &source[open + 1..close]));
            offset = close + 1;
        } else {
            break;
        }
    }
    found
}

fn assertion_open(source: &str, start: usize) -> Option<usize> {
    if source[..start]
        .chars()
        .next_back()
        .is_some_and(is_identifier_character)
    {
        return None;
    }
    let tail = &source[start..];
    let name = ["assert_eq", "assert_ne", "assert"]
        .into_iter()
        .find(|name| {
            tail.strip_prefix(name).is_some_and(|after| {
                !after.chars().next().is_some_and(is_identifier_character)
            })
        })?;
    let after_name = tail[name.len()..].trim_start();
    let after_bang = after_name.strip_prefix('!')?.trim_start();
    after_bang
        .starts_with('(')
        .then_some(source.len() - after_bang.len())
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn matching_paren(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (relative, character) in source[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + relative);
                }
            }
            _ => {}
        }
    }
    None
}

fn identifiers(text: &str) -> Vec<&str> {
    text.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|token| {
            token
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic())
        })
        .collect()
}

fn contains_identifier(text: &str, name: &str) -> bool {
    identifiers(text).contains(&name)
}

fn is_diagnostic(name: &str) -> bool {
    matches!(
        name,
        "stderr" | "stdout" | "error" | "errors" | "message" | "output" | "diagnostic"
    )
}

fn has_rationale(source: &str, start: usize) -> bool {
    source[..start].lines().next_back().is_some_and(|line| {
        line.split_once(RATIONALE)
            .is_some_and(|(_, rationale)| rationale.trim().split_whitespace().count() >= 3)
    })
}

fn line_number(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}
