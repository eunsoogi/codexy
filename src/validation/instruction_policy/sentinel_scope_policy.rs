use std::path::Path;

use crate::paths::display_relative;

const SCOPE_POLICY_CLAUSES: &[&str] = &[
    "MUST review only this issue's acceptance criteria, authorized behavior/files, current PR head or current diff, and necessary regressions.",
    "Every BLOCK finding MUST map to an in-scope acceptance criterion.",
    "Unrelated edge cases MUST be documented as non-blocking follow-up issues and MUST NOT block this lane.",
    "Recurring same-class defects MUST receive one structural root-cause repair rather than phrase patches; MUST ask parent before widening files.",
];

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if !path.ends_with("skills/codex-orchestration/SKILL.md") {
        return;
    }
    report(path, text, errors);
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
        .flat_map(segments)
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

fn segments(sentence: &str) -> Vec<&str> {
    sentence
        .split(';')
        .flat_map(|segment| segment.split(" but "))
        .flat_map(split_modal_and_clause)
        .collect()
}

fn split_modal_and_clause(segment: &str) -> Vec<&str> {
    let Some((left, right)) = segment.split_once(" and ") else {
        return vec![segment];
    };
    if starts_permission(right) {
        vec![left, right]
    } else {
        vec![segment]
    }
}

fn starts_permission(clause: &str) -> bool {
    matches!(
        clause.split_ascii_whitespace().next(),
        Some("may" | "can" | "should" | "must" | "allowed" | "permitted" | "authorized")
    )
}

fn words(sentence: &str) -> Vec<&str> {
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

fn has_positive_permission(words: &[&str], action_index: usize) -> bool {
    words[..action_index].iter().rev().take(8).any(|word| {
        matches!(
            *word,
            "may"
                | "can"
                | "should"
                | "must"
                | "allowed"
                | "permit"
                | "permitted"
                | "authorize"
                | "authorized"
        )
    })
}

fn has_local_prohibition(words: &[&str], action_index: usize) -> bool {
    let context = &words[action_index.saturating_sub(4)..action_index];
    context.windows(2).any(|pair| {
        matches!(pair[0], "must" | "may" | "should") && pair[1] == "not"
            || pair[0] == "not" && matches!(pair[1], "allowed" | "permitted")
    }) || context
        .iter()
        .any(|word| matches!(*word, "cannot" | "can't" | "prohibited"))
}
