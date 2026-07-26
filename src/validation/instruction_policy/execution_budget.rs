use std::path::Path;

use super::clauses::require_all;

const REQUIRED_CLAUSES: &[&str] = &[
    "Every non-trivial child lane MUST declare a finite execution budget before edits begin.",
    "The budget MUST name finite implementation, repair, and reviewer cycle limits.",
    "Continuation MUST consume budget and record either an explicit acceptance criterion newly satisfied or an existing blocker removed.",
    "Every non-trivial parent-owned orchestration stage MUST declare finite implementation, repair, fanout, and reviewer-cycle limits before work begins.",
    "A parent-owned stage MUST NOT use more than three non-Sentinel specialists in total; the packaged Sentinel remains separate.",
    "A repeated parent helper or reviewer cycle MUST record either an explicit acceptance criterion newly satisfied or an existing blocker removed.",
    "Unchanged wait output and full-state replay MUST consume the parent-stage budget. They MUST NOT renew implementation, repair, fanout, or reviewer-cycle limits.",
    "A bounded thread-read fallback that returns oversized preview or history output MUST consume the current parent-stage budget and MUST record only bounded size and token metadata. It MUST NOT renew the stage.",
    "Parent-stage budget enforcement MUST preserve external-wait heartbeat semantics and the packaged Sentinel review gate.",
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
    let mut in_html_comment = false;
    if text
        .lines()
        .any(|line| permits_countermand(line, &mut in_html_comment))
    {
        errors.push(format!(
            "{} execution-budget contract must reject countermanding churn, blocked-goal, and wait policy",
            crate::paths::display_relative(path)
        ));
    }
}

fn permits_countermand(line: &str, in_html_comment: &mut bool) -> bool {
    let Some(policy_text) = policy_text(line, in_html_comment) else {
        return false;
    };
    policy_clauses(&policy_text).into_iter().any(|clause| {
        let words = words(clause);
        permits_parent_cycle_without_progress(&words)
            || (!is_negated(&words)
                && (permits_budget_renewal(&words)
                    || permits_blocked_goal(&words)
                    || permits_wait_progress(&words)))
    })
}

fn policy_text(line: &str, in_html_comment: &mut bool) -> Option<String> {
    let mut remainder = line;
    let mut policy = String::new();
    loop {
        if *in_html_comment {
            let end = remainder.find("-->")?;
            *in_html_comment = false;
            remainder = &remainder[end + 3..];
        } else if let Some(start) = remainder.find("<!--") {
            policy.push_str(&remainder[..start]);
            *in_html_comment = true;
            remainder = &remainder[start + 4..];
        } else {
            policy.push_str(remainder);
            break;
        }
    }
    let trimmed = policy.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let policy = trimmed.trim_start_matches('#').trim_start();
    (!policy.is_empty()).then(|| policy.to_owned())
}

fn policy_clauses(line: &str) -> Vec<&str> {
    line.split(';')
        .flat_map(|clause| clause.split(". "))
        .flat_map(contrast_clauses)
        .collect()
}

fn contrast_clauses(clause: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    for (index, character) in clause.char_indices() {
        if character == ',' {
            if let Some(next_start) = contrast_tail_start(&clause[index + 1..]) {
                clauses.push(&clause[start..index]);
                start = index + 1 + next_start;
            }
        }
    }
    clauses.push(&clause[start..]);
    clauses
}

fn contrast_tail_start(tail: &str) -> Option<usize> {
    let trimmed = tail.trim_start();
    ["but", "and", "while"].iter().find_map(|conjunction| {
        let prefix = trimmed.get(..conjunction.len())?;
        let after_conjunction = trimmed.get(conjunction.len()..)?;
        (prefix.eq_ignore_ascii_case(conjunction)
            && after_conjunction.starts_with(|character: char| character.is_ascii_whitespace()))
        .then(|| tail.len() - after_conjunction.trim_start().len())
    })
}

fn permits_budget_renewal(words: &[String]) -> bool {
    let churn = ["artifact", "file", "diff", "test", "fingerprint"]
        .iter()
        .any(|kind| has_pair(words, kind, "churn"));
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

fn permits_parent_cycle_without_progress(words: &[String]) -> bool {
    let repeat = words
        .iter()
        .position(|word| matches!(word.as_str(), "repeat" | "repeated"));
    let Some(repeat) = repeat else {
        return false;
    };
    let tail = &words[repeat + 1..];
    contains(words, "parent")
        && (contains(words, "helper") || contains(words, "reviewer"))
        && contains(words, "cycle")
        && permits_repeat(words, repeat)
        && denies_all_progress(tail)
}

fn permits_repeat(words: &[String], repeat: usize) -> bool {
    words[..repeat].iter().enumerate().any(|(index, word)| {
        matches!(word.as_str(), "may" | "can" | "must")
            && words.get(index + 1).is_none_or(|next| next != "not")
    })
}

fn denies_all_progress(tail: &[String]) -> bool {
    let criterion = [state(tail, "criterion"), state(tail, "progress")]
        .into_iter()
        .max()
        .unwrap_or(Progress::Absent);
    criterion == Progress::Denied && state(tail, "blocker") == Progress::Denied
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Progress {
    Absent,
    Denied,
    Established,
}

fn state(tail: &[String], subject: &str) -> Progress {
    let Some(index) = tail.iter().position(|word| word == subject) else {
        return Progress::Absent;
    };
    let boundary = tail
        .iter()
        .position(|word| matches!(word.as_str(), "when" | "if" | "with"))
        .unwrap_or(tail.len());
    let locally_denied = tail[..index]
        .iter()
        .rev()
        .take(4)
        .any(|word| matches!(word.as_str(), "no" | "without"));
    if locally_denied || (tail.first().is_some_and(|word| word == "without") && index < boundary) {
        return Progress::Denied;
    }
    let positive = match subject {
        "blocker" => ["removed", "removal"],
        "criterion" => ["satisfied", "newly"],
        _ => ["progress", "progress"],
    };
    tail[index..]
        .iter()
        .take(4)
        .any(|word| word == positive[0] || word == positive[1])
        .then_some(Progress::Established)
        .unwrap_or(Progress::Absent)
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
