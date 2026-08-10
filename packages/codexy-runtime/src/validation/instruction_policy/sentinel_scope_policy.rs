use std::path::Path;

use crate::paths::display_relative;

const SCOPE_POLICY_CLAUSES: &[&str] = &[
    "MUST review only this issue's acceptance criteria, authorized behavior/files, current PR head or current diff, and necessary regressions.",
    "Every BLOCK finding MUST map to an in-scope acceptance criterion.",
    "Unrelated edge cases MUST be documented as non-blocking follow-up issues and MUST NOT block this lane.",
    "Recurring same-class defects MUST receive one structural root-cause repair rather than phrase patches; MUST ask parent before widening files.",
];
const LIVE_OBSERVATION_CLAUSES: &[&str] = &[
    "Live Sentinel observation MUST be read-only and event-driven.",
    "Generic child and ledger polling remains permitted.",
    "Both the child owner and the root orchestrator MUST NOT message, interrupt, replace, duplicate, follow up with, or poll a live Sentinel.",
    "A live Sentinel MUST report its own terminal `PASS`, `BLOCK`, or `UNOBSERVABLE` result naturally.",
];
const NONTERMINAL_OBSERVATION_CLAUSE: &str = "A bounded wait with no event is a non-terminal `PENDING` observation, and an independently observed live reviewer is `RUNNING`; neither observation is a reviewer verdict or fallback-eligible. The owning lane MUST retain the same reviewer and wait for its natural terminal result.";
const LIVE_OBSERVATION_SKILLS: &[&str] = &[
    "skills/orchestration/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/orchestration/references/token-efficient.md",
];
const NONTERMINAL_OBSERVATION_SURFACES: &[&str] = &[
    "skills/orchestration/SKILL.md",
    "skills/orchestration/references/classification-and-control.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/orchestration/references/token-efficient.md",
];

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if path.ends_with("skills/orchestration/SKILL.md") {
        report(path, text, errors);
    }
    let requires_live_policy = LIVE_OBSERVATION_SKILLS
        .iter()
        .any(|skill| path.ends_with(skill));
    let requires_nonterminal_policy = NONTERMINAL_OBSERVATION_SURFACES
        .iter()
        .any(|surface| path.ends_with(surface));
    if !requires_live_policy && !requires_nonterminal_policy {
        return;
    }
    if requires_live_policy
        && LIVE_OBSERVATION_CLAUSES
            .iter()
            .any(|clause| !contains_clause(text, clause))
    {
        errors.push(format!(
            "{} Sentinel scope policy contract failed: missing live-observation clause",
            display_relative(path)
        ));
    }
    if requires_nonterminal_policy && !contains_clause(text, NONTERMINAL_OBSERVATION_CLAUSE) {
        errors.push(format!(
            "{} Sentinel scope policy contract failed: missing non-terminal observation clause",
            display_relative(path)
        ));
    }
    if super::sentinel_live_control::detects(text) {
        errors.push(format!(
            "{} Sentinel scope policy contract failed: must not control a live Sentinel",
            display_relative(path)
        ));
    }
}

fn contains_clause(text: &str, clause: &str) -> bool {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains(&clause.split_whitespace().collect::<Vec<_>>().join(" "))
}

pub(crate) fn check_sentinel(path: &Path, text: &str, errors: &mut Vec<String>) {
    report(path, text, errors);
}

fn report(path: &Path, text: &str, errors: &mut Vec<String>) {
    for violation in violations(text) {
        errors.push(format!(
            "{} Sentinel scope policy contract failed: {violation}",
            display_relative(path)
        ));
    }
}

fn violations(text: &str) -> Vec<&'static str> {
    let mut violations = SCOPE_POLICY_CLAUSES
        .iter()
        .filter(|clause| !text.contains(**clause))
        .map(|_| "missing required scope-policy clause")
        .collect::<Vec<_>>();
    if permits(text, "unrelated edge case", &["block"]) {
        violations.push("must not permit blocking unrelated edge cases");
    }
    if permits(text, "unrelated", &["review"]) {
        violations.push("must not permit review beyond authorized behavior or files");
    }
    if permits(
        text,
        "phrase patch",
        &["use", "resolve", "sufficient", "allow", "permit"],
    ) {
        violations.push("must not permit phrase patches for recurring same-class defects");
    }
    violations
}

fn permits(text: &str, subject: &str, permissions: &[&str]) -> bool {
    text.to_ascii_lowercase()
        .split(['.', '!', '?'])
        .flat_map(|sentence| sentence.split(';'))
        .flat_map(|segment| segment.split(" but "))
        .flat_map(split_modal_and_clause)
        .any(|segment| {
            let words = words(segment);
            segment.contains(subject)
                && words.iter().enumerate().any(|(index, word)| {
                    matches_action(word, permissions)
                        && has_positive_permission(&words, index)
                        && !has_local_prohibition(&words, index)
                })
        })
}

pub(super) fn split_modal_and_clause(segment: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    for (index, _) in segment.match_indices(" and ") {
        let right = &segment[index + " and ".len()..];
        if starts_modal_clause(right) {
            clauses.push(&segment[start..index]);
            start = index + " and ".len();
        }
    }
    clauses.push(&segment[start..]);
    clauses
}

fn starts_modal_clause(clause: &str) -> bool {
    words(clause).iter().any(|word| {
        matches!(
            *word,
            "may" | "can" | "should" | "must" | "allowed" | "permitted" | "authorized"
        )
    })
}

pub(super) fn words(sentence: &str) -> Vec<&str> {
    sentence
        .split(|character: char| !character.is_ascii_alphabetic() && character != '-')
        .filter(|word| !word.is_empty())
        .collect()
}

fn matches_action(word: &str, actions: &[&str]) -> bool {
    actions.iter().any(|action| match *action {
        "block" => word.starts_with("block"),
        "review" => word.starts_with("review"),
        "use" => matches!(word, "use" | "uses" | "used" | "using"),
        "resolve" => word.starts_with("resolv"),
        "sufficient" => word == "sufficient",
        "allow" => word.starts_with("allow"),
        "permit" => word.starts_with("permit"),
        _ => false,
    })
}

pub(super) fn has_positive_permission(words: &[&str], action_index: usize) -> bool {
    words[..action_index].iter().rev().take(8).any(|word| {
        [
            "may",
            "can",
            "should",
            "must",
            "allowed",
            "permit",
            "permitted",
            "authorize",
            "authorized",
        ]
        .contains(word)
    })
}

fn has_local_prohibition(words: &[&str], action_index: usize) -> bool {
    let context = &words[action_index.saturating_sub(4)..action_index];
    context.windows(2).any(|pair| {
        matches!(pair[0], "must" | "may" | "should") && pair[1] == "not"
            || pair[0] == "not" && matches!(pair[1], "allowed" | "permitted")
    }) || context
        .iter()
        .any(|word| matches!(*word, "cannot" | "can't" | "prohibited" | "neither"))
}
