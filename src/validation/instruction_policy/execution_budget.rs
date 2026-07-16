use std::path::Path;

use super::clauses::require_all;

const REQUIRED_CLAUSES: &[&str] = &[
    "Every non-trivial child lane MUST declare a finite execution budget before edits begin.",
    "The budget MUST name finite implementation, repair, and reviewer cycle limits.",
    "Continuation MUST consume budget and record either an explicit acceptance criterion newly satisfied or an existing blocker removed.",
    "File, diff, test, or fingerprint churn without reducing remaining acceptance work MUST NOT renew or reset the budget.",
    "A renewal MUST be an explicit parent-owned new finite budget with recorded acceptance progress or blocker removal.",
    "After all acceptance criteria and required proof are complete, the lane MUST terminate implementation; adjacent findings become non-blocking follow-up candidates.",
    "Budget exhaustion MUST produce one compact terminal parent handoff with current goal/plan, branch/worktree/HEAD, dirty inventory, proof, remaining criteria, and recommended next decision.",
    "Budget exhaustion MUST NOT call `update_goal(blocked)` and MUST NOT weaken external-gate heartbeat semantics.",
    "An external parent heartbeat MUST observe waiting state without messaging the child and MUST send one continuation only on a material transition.",
    "Repeated child waiting turns, goal refreshes, polling, duplicate narrative, unbounded reasoning, or status-only parent receipts MUST consume budget and MUST NOT qualify as acceptance progress.",
    "The execution-budget contract MUST apply to GPT-5.6 Terra child lanes while remaining model-agnostic and MUST NOT hard-code model-specific prose into the state machine.",
];
pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if !path.ends_with("skills/codex-orchestration/references/execution-budget.md") {
        return;
    }
    require_all(
        path,
        text,
        errors,
        "execution-budget contract must preserve finite acceptance-based termination",
        REQUIRED_CLAUSES,
    );
    if text.lines().any(permits_countermand) {
        errors.push(format!(
            "{} execution-budget contract must reject countermanding churn, blocked-goal, and wait policy",
            crate::paths::display_relative(path)
        ));
    }
}

fn permits_countermand(line: &str) -> bool {
    let Some(policy_text) = policy_text(line) else {
        return false;
    };
    policy_clauses(policy_text).into_iter().any(|clause| {
        let words = words(clause);
        !is_negated(&words)
            && (permits_budget_renewal(&words)
                || permits_blocked_goal(&words)
                || permits_wait_progress(&words))
    })
}

fn policy_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("<!--") {
        return None;
    }
    let Some(after_marker) = trimmed.strip_prefix('#') else {
        return Some(line);
    };
    if after_marker.starts_with(|character: char| character.is_ascii_digit()) {
        return Some(after_marker);
    }
    None
}

fn policy_clauses(line: &str) -> Vec<&str> {
    line.split(';').flat_map(contrast_clauses).collect()
}

fn contrast_clauses(clause: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    for (index, character) in clause.char_indices() {
        if character == ','
            && let Some(next_start) = contrast_tail_start(&clause[index + 1..])
        {
            clauses.push(&clause[start..index]);
            start = index + 1 + next_start;
        }
    }
    clauses.push(&clause[start..]);
    clauses
}

fn contrast_tail_start(tail: &str) -> Option<usize> {
    let trimmed = tail.trim_start();
    let prefix = trimmed.get(..3)?;
    let after_but = trimmed.get(3..)?;
    if prefix.eq_ignore_ascii_case("but")
        && after_but.starts_with(|character: char| character.is_ascii_whitespace())
    {
        Some(tail.len() - after_but.trim_start().len())
    } else {
        None
    }
}

fn permits_budget_renewal(words: &[String]) -> bool {
    let churn = has_pair(words, "artifact", "churn");
    let wait_refresh = has_pair(words, "wait", "refresh") || has_pair(words, "wait", "refreshes");
    let child_self = contains(words, "child") && contains(words, "self");
    (churn || wait_refresh || child_self)
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "renew" | "reset"))
        && permits(words)
}

fn permits_blocked_goal(words: &[String]) -> bool {
    ["budget", "exhaustion", "update", "goal", "blocked"]
        .iter()
        .all(|word| contains(words, word))
        && permits(words)
}

fn permits_wait_progress(words: &[String]) -> bool {
    contains(words, "repeated")
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "wait" | "waiting" | "refresh" | "refreshes"))
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "qualify" | "qualifies"))
        && contains(words, "progress")
        && permits(words)
}

fn permits(words: &[String]) -> bool {
    words.iter().enumerate().any(|(index, word)| {
        matches!(word.as_str(), "may" | "can" | "must")
            && words.get(index + 1).is_none_or(|next| next != "not")
    })
}

fn is_negated(words: &[String]) -> bool {
    words.windows(2).any(|pair| {
        matches!(pair, [first, second] if matches!(first.as_str(), "may" | "can" | "must") && second == "not")
            || matches!(pair, [first, second] if first == "not" && matches!(second.as_str(), "allowed" | "permitted"))
    }) || words
        .iter()
        .any(|word| matches!(word.as_str(), "forbidden" | "prohibited"))
}

fn has_pair(words: &[String], first: &str, second: &str) -> bool {
    words
        .windows(2)
        .any(|pair| pair[0] == first && pair[1] == second)
}

fn contains(words: &[String], value: &str) -> bool {
    words.iter().any(|word| word == value)
}

fn words(line: &str) -> Vec<String> {
    line.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}
