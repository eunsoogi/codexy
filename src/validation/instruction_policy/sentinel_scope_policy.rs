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
    "Both the child owner and the root orchestrator MUST NOT message, interrupt, replace, follow up with, or poll a live Sentinel.",
    "A live Sentinel MUST report its own terminal `PASS`, `BLOCK`, or `UNOBSERVABLE` result naturally.",
];
const LIVE_OBSERVATION_SKILLS: &[&str] = &[
    "skills/codex-orchestration/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/token-efficient-orchestration/SKILL.md",
];

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if path.ends_with("skills/codex-orchestration/SKILL.md") {
        report(path, text, errors);
    }
    if !LIVE_OBSERVATION_SKILLS
        .iter()
        .any(|skill| path.ends_with(skill))
    {
        return;
    }
    if LIVE_OBSERVATION_CLAUSES
        .iter()
        .any(|clause| !contains_clause(text, clause))
    {
        errors.push(format!(
            "{} Sentinel scope policy contract failed: missing live-observation clause",
            display_relative(path)
        ));
    }
    if live_sentinel_control(text) {
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

fn live_sentinel_control(text: &str) -> bool {
    let mut fenced = false;
    text.lines().any(|line| {
        let line = line.trim();
        if line.starts_with("```") {
            fenced = !fenced;
            return false;
        }
        if fenced || line.starts_with("sentinel_") {
            return false;
        }
        line.to_ascii_lowercase()
            .split(['.', '!', '?'])
            .flat_map(|sentence| sentence.split(" but "))
            .flat_map(split_modal_and_clause)
            .any(|clause| {
                let words = words(clause);
                clause.contains("live sentinel")
                    && !historical_or_terminal(clause)
                    && words.iter().enumerate().any(|(index, word)| {
                        matches_live_control(&words, word)
                            && has_positive_permission(&words, index)
                            && !has_live_control_prohibition(&words, index)
                    })
            })
    })
}

fn historical_or_terminal(sentence: &str) -> bool {
    "historic|former|previous|terminal pass|terminal block|terminal unobservable"
        .split('|')
        .any(|marker| sentence.contains(marker))
}

fn matches_live_control(words: &[&str], word: &str) -> bool {
    ["message", "interrupt", "replace", "follow", "follow-up"].contains(&word)
        || word.starts_with("poll")
        || word == "send" && words.contains(&"terminal-status")
}

fn has_live_control_prohibition(words: &[&str], action_index: usize) -> bool {
    let context = words[..action_index]
        .rsplit(|word| *word == "but")
        .next()
        .unwrap();
    context
        .windows(2)
        .any(|pair| matches!(pair[0], "must" | "may" | "should") && pair[1] == "not")
        || context
            .iter()
            .any(|word| matches!(*word, "never" | "refrain" | "neither"))
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

fn split_modal_and_clause(segment: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    for (index, _) in segment.match_indices(" and ") {
        let right = &segment[index + " and ".len()..];
        if starts_permission(right) {
            clauses.push(&segment[start..index]);
            start = index + " and ".len();
        }
    }
    clauses.push(&segment[start..]);
    clauses
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

#[cfg(test)]
mod tests {
    use super::live_sentinel_control;

    #[test]
    fn applies_clause_local_live_sentinel_polarity() {
        for text in [
            "Root MUST NOT ignore safety, but MAY poll a live Sentinel.",
            "Root MAY send a terminal-status request to a live Sentinel.",
        ] {
            assert!(live_sentinel_control(text), "{text}");
        }
        for text in [
            "Root MUST never poll a live Sentinel.",
            "Root MUST refrain from polling a live Sentinel.",
        ] {
            assert!(!live_sentinel_control(text), "{text}");
        }
    }
}
