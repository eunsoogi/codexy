use serde_json::Value;

use super::codex_review_handoff::has_negative_label_value;

pub(super) fn check_completion_handoff(handoff: &str, pr_state: &str) -> Vec<String> {
    if !claims_label_guarded_handoff(handoff) {
        return Vec::new();
    }
    let pr_state = match serde_json::from_str::<Value>(pr_state) {
        Ok(value) => value,
        Err(error) => return vec![format!("GitHub label PR state JSON error: {error}")],
    };
    if !is_open_pr(&pr_state) {
        return Vec::new();
    }
    if !is_codexy_lane(&pr_state) {
        return Vec::new();
    }
    if has_label_consideration_evidence(handoff) {
        return Vec::new();
    }
    let mut errors = Vec::new();
    check_label_evidence(
        "PR labels",
        &label_names(pr_state.get("labels")),
        &mut errors,
    );
    match issue_nodes(pr_state.get("closingIssuesReferences")) {
        issues if !issues.is_empty() => {
            for issue in issues {
                let number = issue
                    .get("number")
                    .and_then(Value::as_u64)
                    .map_or_else(|| "<unknown>".to_owned(), |number| format!("#{number}"));
                check_label_evidence(
                    &format!("issue {number} labels"),
                    &label_names(issue.get("labels")),
                    &mut errors,
                );
            }
        }
        _ => errors
            .push("GitHub label evidence missing closingIssuesReferences with issue labels".into()),
    }
    errors
}

fn claims_label_guarded_handoff(handoff: &str) -> bool {
    claims_pr_readiness(handoff) || claims_completion(handoff)
}

fn check_label_evidence(surface: &str, labels: &[String], errors: &mut Vec<String>) {
    if labels.is_empty() {
        errors.push(format!("{surface} missing label application evidence"));
    }
}

fn label_names(labels: Option<&Value>) -> Vec<String> {
    match labels {
        Some(Value::Array(items)) => items.iter().filter_map(label_name).collect(),
        Some(Value::Object(map)) => map
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(label_name)
            .collect(),
        _ => Vec::new(),
    }
}

fn label_name(value: &Value) -> Option<String> {
    value
        .as_str()
        .or_else(|| value.get("name").and_then(Value::as_str))
        .map(str::to_owned)
}

fn issue_nodes(issues: Option<&Value>) -> Vec<&Value> {
    match issues {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(Value::Object(map)) => map
            .get("nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect(),
        _ => Vec::new(),
    }
}

fn is_open_pr(pr_state: &Value) -> bool {
    pr_state
        .get("state")
        .and_then(Value::as_str)
        .is_some_and(|state| state.eq_ignore_ascii_case("OPEN"))
}

fn is_codexy_lane(pr_state: &Value) -> bool {
    string_field(pr_state, &["repository", "nameWithOwner", "headRepository"])
        .iter()
        .any(|value| value == "eunsoogi/codexy")
        || string_field(pr_state, &["url"]).iter().any(|value| {
            value.contains("github.com/eunsoogi/codexy/")
                || value.ends_with("github.com/eunsoogi/codexy")
        })
}

fn string_field(value: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .filter_map(|key| value.get(*key).and_then(Value::as_str))
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn has_label_consideration_evidence(handoff: &str) -> bool {
    handoff.lines().any(|line| {
        let line = line.to_ascii_lowercase();
        ["labels considered", "label consideration"]
            .into_iter()
            .any(|phrase| line.contains(phrase))
            && [
                "no matching",
                "no-match",
                "no applicable",
                "not applicable",
                "not-applicable",
            ]
            .into_iter()
            .any(|phrase| line.contains(phrase))
            && ![
                "missing",
                "empty",
                "absent",
                "not applied",
                "without",
                "no labels",
            ]
            .into_iter()
            .any(|phrase| line.contains(phrase))
    })
}

fn claims_pr_readiness(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    [
        "merge-ready",
        "merge-readiness",
        "merge readiness",
        "merge ready",
        "ready to merge",
        "ready for merge",
        "pr-ready",
        "pr-readiness",
        "pr readiness",
        "pr ready",
        "pr is ready",
        "pull request is ready",
    ]
    .into_iter()
    .any(|phrase| has_unnegated_readiness_phrase(&text, phrase, 24))
}

fn claims_completion(handoff: &str) -> bool {
    let mut text = handoff.to_ascii_lowercase();
    if has_not_complete_until_merge(&text) {
        text = text.replace("verification completed.", "verification evidence.");
        text = text.replace("verification completed:", "verification evidence:");
        for phrase in [
            "successfully completed",
            "completed successfully",
            "completed",
            "finished",
            "finalized",
        ] {
            text = text.replace(&format!("verification {phrase};"), "verification evidence;");
        }
    }
    ["completed", "finished", "finalized", "all set"]
        .iter()
        .any(|phrase| has_unnegated_phrase(&text, phrase, 16))
        || ["done", "complete", "completes", "finish", "finalize"]
            .iter()
            .any(|word| has_unnegated_phrase(&text, word, 16))
}

fn has_not_complete_until_merge(text: &str) -> bool {
    "not complete until merge|not currently complete until merge"
        .split('|')
        .any(|phrase| has_unnegated_phrase(text, phrase, 16))
}

fn has_unnegated_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let after_index = absolute_index + phrase.len();
        if is_boundary(text[..absolute_index].chars().next_back())
            && is_boundary(text[after_index..].chars().next())
            && !has_nearby_negation(
                &text[char_window_start(text, absolute_index, negation_window)..absolute_index],
            )
        {
            return true;
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}

fn has_unnegated_readiness_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let after_index = absolute_index + phrase.len();
        if is_boundary(text[..absolute_index].chars().next_back())
            && is_boundary(text[after_index..].chars().next())
            && !has_nearby_negation(
                &text[char_window_start(text, absolute_index, negation_window)..absolute_index],
            )
            && !has_negative_label_value(&text[after_index..])
        {
            return true;
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}

fn has_nearby_negation(prefix: &str) -> bool {
    "no|not|not yet|not currently|without|isn't|is not"
        .split('|')
        .any(|phrase| prefix.trim_end().ends_with(phrase))
}

fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}
