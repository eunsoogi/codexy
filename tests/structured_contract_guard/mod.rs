#![allow(dead_code)]

mod sanitize;

use std::{collections::HashMap, fs, path::Path, process::Command};

use sanitize::{sanitize, strip_comments};

const RATIONALE: &str = "structured-contract: non-contract substring rationale:";

pub(crate) fn comparison_counts(
    relative_paths: &[&str],
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut before = 0;
    let mut after = 0;
    for relative in relative_paths {
        let current = fs::read_to_string(root.join(relative))?;
        let base = Command::new("git")
            .args(["show", &format!("origin/main:{relative}")])
            .current_dir(root)
            .output()?;
        if !base.status.success() {
            return Err(format!("missing origin/main:{relative}").into());
        }
        before += scan_source(&String::from_utf8(base.stdout)?).len();
        after += scan_source(&current).len();
    }
    Ok((before, after))
}

pub(crate) fn repository_violations() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("git")
        .args(["diff", "--name-only", "origin/main", "--", "tests"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("git diff failed for migration guard".into());
    }
    let mut violations = Vec::new();
    for relative in String::from_utf8(output.stdout)?
        .lines()
        .filter(|path| path.ends_with(".rs"))
    {
        let current_path = root.join(relative);
        if !current_path.is_file() {
            continue;
        }
        let current = fs::read_to_string(current_path)?;
        let base = Command::new("git")
            .args(["show", &format!("origin/main:{relative}")])
            .current_dir(root)
            .output()?;
        let base_text = base
            .status
            .success()
            .then(|| String::from_utf8_lossy(&base.stdout));
        let base_violations = base_text.as_deref().map(scan_source).unwrap_or_default();
        let mut allowed = counts(&base_violations);
        for violation in scan_source(&current) {
            let remaining = allowed.entry(violation.clone()).or_default();
            if *remaining == 0 {
                violations.push(format!("{relative}: {violation}"));
            } else {
                *remaining -= 1;
            }
        }
    }
    Ok(violations)
}

fn counts(violations: &[String]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for violation in violations {
        *counts.entry(violation.clone()).or_default() += 1;
    }
    counts
}

pub(crate) fn scan_source(source: &str) -> Vec<String> {
    let clean = sanitize(source);
    let provenance = strip_comments(source);
    let governed = governed_bindings(&provenance);
    assertions(&clean)
        .into_iter()
        .filter_map(|(start, body)| {
            if !has_contains_call(body) {
                return None;
            }
            let identifiers = identifiers(body);
            let governed_receiver = identifiers
                .iter()
                .find(|name| governed.iter().any(|bound| bound == **name));
            if let Some(receiver) = governed_receiver {
                return Some(format!(
                    "line {} receiver `{receiver}`",
                    line_number(source, start)
                ));
            }
            if identifiers.iter().any(|name| is_diagnostic(name)) || has_rationale(source, start) {
                return None;
            }
            let receiver = identifiers.first()?;
            Some(format!(
                "line {} receiver `{receiver}`",
                line_number(source, start)
            ))
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

fn assertions(source: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut offset = 0;
    while let Some(relative) = source[offset..].find("assert") {
        let start = offset + relative;
        let tail = &source[start..];
        let Some(bang) = tail.find('!') else { break };
        let name = tail[..bang].trim();
        if !matches!(name, "assert" | "assert_eq" | "assert_ne") {
            offset = start + bang + 1;
            continue;
        }
        let Some(open) = tail[bang + 1..]
            .find('(')
            .map(|index| start + bang + 1 + index)
        else {
            break;
        };
        if let Some(close) = matching_paren(source, open) {
            found.push((start, &source[open + 1..close]));
            offset = close + 1;
        } else {
            break;
        }
    }
    found
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
