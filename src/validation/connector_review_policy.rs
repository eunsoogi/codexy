use std::path::Path;

use crate::paths::display_relative;

use super::review_response_cluster::instruction_source;

const REFERENCE_PATH: &str = "skills/git-workflow/references/codex-connector-review.md";
const HEADING: &str = "## Required Procedure";
const OBLIGATIONS: [(&str, &str); 7] = [
    (
        "automatic-disabled",
        "codex connector automatic review must remain disabled",
    ),
    (
        "proof-ci-before-review",
        "before requesting review parent orchestrator must complete local affected proof and wait for required ci readiness on the frozen exact head",
    ),
    (
        "exactly-one-review",
        "after local proof and required ci readiness parent orchestrator must request exactly one @codex review after an owning child sentinel pass on a frozen exact head and before merge",
    ),
    (
        "wait-batch",
        "parent orchestrator must wait for the requested review s terminal output and batch every actionable connector finding into one repair cycle",
    ),
    (
        "child-repair-sentinel",
        "owning child must repair the batch and run a fresh packaged sentinel on the repaired exact head without requesting another connector review",
    ),
    (
        "no-automatic-or-duplicate",
        "automatic per push duplicate unchanged head and piecemeal codex connector review requests must not be made",
    ),
    (
        "material-expansion-exception",
        "another connector review must not be requested unless a maintainer explicitly authorizes it after material scope expansion",
    ),
];

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if !path.ends_with(REFERENCE_PATH) {
        return;
    }
    let normative = instruction_source::normative_markdown(text);
    let obligations = procedure_obligations(&normative, path, errors);
    for (id, expected) in OBLIGATIONS {
        if obligations.get(id).map(String::as_str) != Some(expected) {
            errors.push(format!(
                "{} Codex connector review policy is missing required obligation [{id}]",
                display_relative(path)
            ));
        }
    }
    for sentence in normative.split(['.', '\n']) {
        if active_request_variant(sentence) {
            errors.push(format!(
                "{} Codex connector review policy reintroduces an automatic or repeated review",
                display_relative(path)
            ));
            break;
        }
    }
}

fn procedure_obligations(
    normative: &str,
    path: &Path,
    errors: &mut Vec<String>,
) -> std::collections::BTreeMap<String, String> {
    let mut in_procedure = false;
    let mut obligations = std::collections::BTreeMap::new();
    for line in normative.lines().map(str::trim) {
        if line == HEADING {
            in_procedure = true;
            continue;
        }
        if in_procedure && (line.starts_with("# ") || line.starts_with("## ")) {
            break;
        }
        if !in_procedure {
            continue;
        }
        let Some((id, clause)) = obligation(line) else {
            continue;
        };
        if obligations
            .insert(id.to_owned(), normalize(clause))
            .is_some()
        {
            errors.push(format!(
                "{} Codex connector review policy duplicates obligation [{id}]",
                display_relative(path)
            ));
        }
    }
    obligations
}

fn obligation(line: &str) -> Option<(&str, &str)> {
    let (_, content) = line.split_once(". ")?;
    let content = content.strip_prefix('[')?;
    let (id, clause) = content.split_once("] ")?;
    (!id.is_empty() && !clause.is_empty()).then_some((id, clause))
}

fn active_request_variant(sentence: &str) -> bool {
    let sentence = normalize(sentence);
    let positive_must = sentence.contains(" must ")
        && !sentence.contains(" must not ")
        && !sentence.contains("without request");
    let request = sentence.contains("request") && sentence.contains("review");
    let automatic_enable = sentence.contains("automatic")
        && sentence.contains("review")
        && (sentence.contains("enable") || sentence.contains("enabled"));
    let repeated = [
        "every push",
        "each push",
        "per push",
        "on push",
        "duplicate",
        "another",
        "second",
        "piecemeal",
        "after every repair",
    ]
    .iter()
    .any(|marker| sentence.contains(marker));
    positive_must && ((request && (sentence.contains("automatic") || repeated)) || automatic_enable)
}

fn normalize(text: &str) -> String {
    text.chars()
        .map(|character| {
            (character.is_ascii_alphanumeric() || character == '@')
                .then_some(character)
                .unwrap_or(' ')
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
