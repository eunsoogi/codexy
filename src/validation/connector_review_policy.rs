use std::path::Path;

use crate::paths::display_relative;

mod active_scope;

const REFERENCE_PATH: &str = "skills/git-workflow/references/codex-connector-review.md";
const SKILL_PATH: &str = "skills/git-workflow/SKILL.md";
const AGENTS_PATH: &str = "AGENTS.md";
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
    let active = active_scope::lines(text);
    if path.ends_with(AGENTS_PATH) {
        require(
            path,
            &active,
            &["codex connector automatic review", "must", "disabled"],
            errors,
        );
        require(
            path,
            &active,
            &["must", "request", "explicit", "@codex", "review"],
            errors,
        );
    } else if path.ends_with(SKILL_PATH) {
        require(
            path,
            &active,
            &["references/codex-connector-review.md"],
            errors,
        );
        require(
            path,
            &active,
            &["before merge", "parent/orchestrator", "must", "follow"],
            errors,
        );
    } else if path.ends_with(REFERENCE_PATH) {
        reference_contract(path, &active, errors);
    } else {
        return;
    }
    if active.iter().any(|line| {
        line.split(['.', ';', ':'])
            .any(active_request_variant_in_fragment)
    }) {
        errors.push(format!(
            "{} Codex connector review policy reintroduces an automatic or repeated review",
            display_relative(path)
        ));
    }
}

fn require(path: &Path, active: &[String], terms: &[&str], errors: &mut Vec<String>) {
    if !active.iter().any(|line| {
        terms.iter().all(|term| {
            line.to_ascii_lowercase()
                .contains(&term.to_ascii_lowercase())
        })
    }) {
        errors.push(format!(
            "{} Codex connector review policy is missing an active governed-surface obligation",
            display_relative(path)
        ));
    }
}

fn reference_contract(path: &Path, active: &[String], errors: &mut Vec<String>) {
    let obligations = procedure_obligations(active, path, errors);
    for (id, expected) in OBLIGATIONS {
        if obligations.get(id).map(String::as_str) != Some(expected) {
            errors.push(format!(
                "{} Codex connector review policy is missing required obligation [{id}]",
                display_relative(path)
            ));
        }
    }
}

fn procedure_obligations(
    active: &[String],
    path: &Path,
    errors: &mut Vec<String>,
) -> std::collections::BTreeMap<String, String> {
    let mut in_procedure = false;
    let mut obligations = std::collections::BTreeMap::new();
    let mut next_number = 1;
    for line in active.iter().map(String::as_str) {
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
        let Some((number, id, clause)) = obligation(line) else {
            continue;
        };
        let expected = OBLIGATIONS.iter().position(|(expected, _)| *expected == id);
        if number != next_number || expected.is_none_or(|index| number != index + 1) {
            errors.push(format!(
                "{} Codex connector review policy has an invalid obligation number or ID",
                display_relative(path)
            ));
            continue;
        }
        next_number += 1;
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

fn obligation(line: &str) -> Option<(usize, &str, &str)> {
    let (number, content) = line.split_once(". ")?;
    let number = number.parse::<usize>().ok()?;
    let content = content.strip_prefix('[')?;
    let (id, clause) = content.split_once("] ")?;
    (!id.is_empty() && !clause.is_empty()).then_some((number, id, clause))
}

fn active_request_variant_in_fragment(fragment: &str) -> bool {
    let fragment = normalize(&fragment.replace(',', " comma "));
    let words = fragment.split_whitespace().collect::<Vec<_>>();
    let modal_positions = words
        .iter()
        .enumerate()
        .filter_map(|(index, word)| (*word == "must").then_some(index))
        .collect::<Vec<_>>();
    let Some(&first_modal) = modal_positions.first() else {
        return false;
    };
    let mut subject = &words[..first_modal];
    for (position, start) in modal_positions.iter().copied().enumerate() {
        if position > 0 {
            let previous = modal_positions[position - 1];
            let between = &words[previous + 1..start];
            let boundary = between
                .iter()
                .position(|word| *word == "comma")
                .or_else(|| {
                    between
                        .iter()
                        .rposition(|word| matches!(*word, "and" | "or" | "but" | "then"))
                });
            if let Some(boundary) = boundary {
                let candidate = &between[boundary + 1..];
                if !candidate.is_empty() {
                    subject = candidate;
                }
            }
        }
        let end = modal_positions
            .get(position + 1)
            .copied()
            .unwrap_or(words.len());
        let clause = &words[start..end];
        let positive_must = clause.get(1) != Some(&"not")
            && !clause
                .windows(2)
                .any(|pair| pair[0] == "without" && pair[1].starts_with("request"));
        let scoped = || subject.iter().chain(clause.iter()).copied();
        let request = scoped().any(|word| word.starts_with("request"))
            && scoped().any(|word| word.starts_with("review"));
        let automatic_enable = scoped().any(|word| word == "automatic")
            && scoped().any(|word| word.starts_with("review"))
            && ["enable", "enabled", "configure", "configured"]
                .iter()
                .any(|verb| clause.contains(verb));
        let repeated = [
            "every push",
            "each push",
            "per push",
            "on push",
            "duplicate",
            "another",
            "second",
            "repeated",
            "piecemeal",
            "after every repair",
        ]
        .iter()
        .any(|marker| clause.join(" ").contains(marker));
        if positive_must
            && ((request && (scoped().any(|word| word == "automatic") || repeated))
                || automatic_enable)
        {
            return true;
        }
    }
    false
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
