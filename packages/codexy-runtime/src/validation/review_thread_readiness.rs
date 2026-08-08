use serde_json::Value;

const MISSING_REVIEW_THREADS: &str = "PR readiness missing reviewThreads.nodes PR state evidence";
const READY_PHRASES: &[&str] = &[
    "merge-ready",
    "merge-readiness",
    "merge readiness",
    "merge ready",
    "ready to merge",
    "ready for merge",
    "ready for parent handoff",
    "ready for handoff",
    "parent-handoff-ready",
    "parent handoff ready",
    "pr-ready",
    "pr-readiness",
    "pr readiness",
    "pr ready",
    "pr is ready",
    "pull-request-ready",
    "pull request ready",
    "pull request is ready",
    "parent can open pr next",
    "parent can merge",
];

pub(super) fn check_handoff(handoff: &str, pr_state: &Value) -> Option<String> {
    let text = handoff.to_ascii_lowercase();
    if !claims_ready(&text) {
        return None;
    }
    let Some(threads) = pr_state.get("reviewThreads") else {
        return Some(MISSING_REVIEW_THREADS.into());
    };
    if threads.get("nodes").and_then(Value::as_array).is_none() {
        return Some(MISSING_REVIEW_THREADS.into());
    }
    if let Some(error) = super::review_thread_evidence::check(threads) {
        return Some(error);
    }
    check(pr_state)
}

pub(super) fn check(pr_state: &Value) -> Option<String> {
    let thread = pr_state
        .get("reviewThreads")
        .and_then(|threads| threads.get("nodes"))
        .and_then(Value::as_array)?
        .iter()
        .find(|thread| thread.get("isResolved").and_then(Value::as_bool) == Some(false))?;
    Some(format!(
        "unresolved review thread remains before PR-ready or merge-ready claims: {}; resolve fixed or accepted threads after thread-state verification",
        thread_label(thread)
    ))
}

fn claims_ready(text: &str) -> bool {
    let current = super::readiness_context::current_text(text);
    READY_PHRASES
        .iter()
        .any(|phrase| has_affirmed_phrase(&current, phrase))
}

fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if is_boundary(text[..start].chars().next_back())
            && is_boundary(text[end..].chars().next())
            && !is_locally_negated(&text[..start])
            && !has_blocking_label_value(&text[end..])
            && !super::handoff_claims::has_negative_label_value(&text[end..])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn has_blocking_label_value(suffix: &str) -> bool {
    let suffix = suffix.trim_start_matches([' ', '\t']);
    let Some((label, value)) = suffix.split_once(':') else {
        return false;
    };
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '-', '*']);
    match label.trim() {
        "" => has_blocking_status_value(value),
        "blocker" | "blockers" => {
            !starts_with_any(value, &["none", "no", "no blocker", "no blockers", "clear"])
        }
        "status" => has_blocking_status_value(value),
        _ => false,
    }
}

fn has_blocking_status_value(value: &str) -> bool {
    !starts_with_any(
        value,
        &["ready", "complete", "completed", "passed", "clean"],
    ) && (super::handoff_claims::has_negative_label_value(&format!(": {value}"))
        || starts_with_any(
            value,
            &[
                "blocked",
                "blocking",
                "waiting",
                "pending",
                "unresolved",
                "incomplete",
                "not complete",
                "not yet complete",
            ],
        ))
}

fn starts_with_any(value: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| {
        value.strip_prefix(phrase).is_some_and(|rest| {
            rest.chars()
                .next()
                .is_none_or(|character| !character.is_ascii_alphanumeric())
        })
    })
}

fn is_locally_negated(prefix: &str) -> bool {
    let clause = prefix
        .rsplit_once(['.', '!', '?', ';', ':', ',', '\n'])
        .map_or(prefix, |(_, clause)| clause);
    clause
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| {
            matches!(
                word,
                "no" | "not"
                    | "never"
                    | "without"
                    | "isn't"
                    | "wasn't"
                    | "hasn't"
                    | "haven't"
                    | "aren't"
                    | "don't"
                    | "doesn't"
                    | "didn't"
                    | "won't"
                    | "can't"
                    | "cannot"
            )
        })
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn thread_label(thread: &Value) -> String {
    let id = thread
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown thread");
    let path = thread
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("unknown path");
    let url = thread
        .get("comments")
        .and_then(|comments| comments.get("nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|comments| comments.iter())
        .find_map(|comment| comment.get("url").and_then(Value::as_str))
        .unwrap_or("no comment URL");
    format!("{id} at {path} ({url})")
}
