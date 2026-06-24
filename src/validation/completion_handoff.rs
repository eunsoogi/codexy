use serde_json::Value;
pub(super) fn check(handoff: &str, pr_state: &str) -> Vec<String> {
    let pr_state = match serde_json::from_str::<Value>(pr_state) {
        Ok(value) => value,
        Err(error) => return vec![format!("completion handoff PR state JSON error: {error}")],
    };
    if let Some(error) = pr_state_input_error(&pr_state) {
        return vec![error];
    }
    let review_thread_errors = super::review_thread_resolution::check(handoff, &pr_state);
    if !review_thread_errors.is_empty() {
        return review_thread_errors;
    }
    let codex_review_errors = super::codex_review_handoff::check(handoff, &pr_state);
    if !codex_review_errors.is_empty() {
        return codex_review_errors;
    }
    if !is_open_pr(&pr_state) || !claims_completion(handoff) || states_explicit_deferral(handoff) {
        return Vec::new();
    }
    vec![format!(
        "opening a PR is not completion: PR #{} is still open; state an explicit stop, wait, draft-only, leave-open, or no-merge deferral instead of claiming completion",
        pr_number(&pr_state)
    )]
}
fn is_open_pr(pr_state: &Value) -> bool {
    string_field(pr_state, "state").is_some_and(|state| state.eq_ignore_ascii_case("OPEN"))
}
fn claims_completion(handoff: &str) -> bool {
    let mut text = handoff.to_ascii_lowercase();
    if has_unnegated_phrase(&text, "not complete until merge", 16) {
        text = text.replace("verification completed.", "verification evidence.");
        text = text.replace("verification completed:", "verification evidence:");
        for phrase in
            "successfully completed|completed successfully|completed|finished|finalized".split('|')
        {
            text = text.replace(&format!("verification {phrase};"), "verification evidence;");
        }
    }
    ["completed", "finished", "finalized", "all set"]
        .iter()
        .any(|phrase| has_unnegated_phrase(&text, phrase, 16))
        || has_unnegated_word(&text, "done", 16)
        || has_unnegated_word(&text, "complete", 16)
        || has_unnegated_word(&text, "completes", 16)
        || has_unnegated_word(&text, "finish", 16)
        || has_unnegated_word(&text, "finalize", 16)
}
fn states_explicit_deferral(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    "maintainer requested stop|maintainer requested wait|maintainer requested no merge|maintainer requested no-merge|maintainer requested push only|maintainer requested leave open|maintainer requested leave-open|maintainer explicitly requested stop|maintainer explicitly requested wait|maintainer explicitly requested no merge|maintainer explicitly requested no-merge|maintainer explicitly requested push only|maintainer explicitly requested leave open|maintainer explicitly requested leave-open|maintainer asked me to stop|maintainer asked me to wait|maintainer asked me to leave open|do not merge per maintainer|no merge per maintainer|no-merge instruction|maintainer requested draft-only|maintainer requested draft only|maintainer explicitly requested draft-only|maintainer explicitly requested draft only|draft pr per maintainer|draft pull request per maintainer|draft-only instruction|leave open per maintainer|left open per maintainer|deferred by maintainer"
        .split('|')
        .any(|phrase| has_unnegated_deferral_phrase(&text, phrase, 80))
}
fn pr_state_input_error(pr_state: &Value) -> Option<String> {
    if let Some(field) = ["state", "mergeStateStatus"]
        .iter()
        .find(|field| string_field(pr_state, field).is_none())
    {
        return Some(format!(
            "completion handoff PR state missing required field: {field}"
        ));
    }
    if pr_state.get("isDraft").and_then(Value::as_bool).is_none() {
        return Some("completion handoff PR state missing required field: isDraft".into());
    }
    None
}
fn has_unnegated_deferral_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let after_index = absolute_index + phrase.len();
        if phrase_has_boundaries(text, absolute_index, after_index)
            && !has_unchecked_checklist_marker_before(text, absolute_index)
            && !has_false_deferral_label(text, phrase, absolute_index, after_index)
        {
            let prefix_start = char_window_start(text, absolute_index, negation_window);
            if !has_nearby_negation(&text[prefix_start..absolute_index]) {
                return true;
            }
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}
fn has_unchecked_checklist_marker_before(text: &str, start: usize) -> bool {
    text[..start]
        .trim_end_matches([' ', '\t', '*'])
        .ends_with("- [ ]")
}
fn has_false_deferral_label(text: &str, phrase: &str, start: usize, after_index: usize) -> bool {
    let suffix = text[after_index..].trim_start_matches([' ', '\t']);
    if (has_false_label_value(suffix)
        && !suffix.strip_prefix("no").is_some_and(starts_with_boundary))
        || suffix.starts_with("from maintainer was not requested")
        || (suffix
            .strip_prefix("was requested")
            .is_some_and(starts_with_boundary)
            && !suffix.starts_with("was requested by maintainer"))
        || ["is not requested", "was not requested"]
            .iter()
            .any(|phrase| {
                suffix
                    .strip_prefix(phrase)
                    .is_some_and(starts_with_boundary)
            })
        || ['?', '\n', '=', '-'].iter().any(|prefix| {
            suffix
                .strip_prefix(*prefix)
                .map(str::trim_start)
                .is_some_and(has_false_label_value)
        })
        || (text[char_window_start(text, start, 80)..start]
            .trim_start()
            .starts_with("no explicit")
            && suffix
                .strip_prefix("was requested")
                .is_some_and(starts_with_boundary))
    {
        return true;
    }
    let Some(value) = suffix.strip_prefix(':') else {
        if phrase.ends_with("instruction") {
            return !suffix
                .strip_prefix("was requested by maintainer")
                .or_else(|| suffix.strip_prefix("per maintainer"))
                .is_some_and(starts_with_boundary);
        }
        return false;
    };
    let value = value.trim_start_matches([' ', '\t']);
    if matches!(value.chars().next(), None | Some('\n' | '\r' | '.' | ';')) {
        return true;
    }
    if phrase.ends_with("instruction")
        && !["maintainer requested", "maintainer asked", "per maintainer"]
            .iter()
            .any(|phrase| has_unnegated_phrase(value, phrase, 16))
        || (phrase == "no-merge instruction" && !value.contains("no merge")
            || phrase == "draft-only instruction" && !value.contains("draft"))
    {
        return true;
    }
    has_false_label_value(value)
}
fn has_false_label_value(value: &str) -> bool {
    [
        "none",
        "false",
        "not requested",
        "not required",
        "not applicable",
        "not-applicable",
        "n/a",
        "na",
        "no",
    ]
    .iter()
    .any(|word| value.strip_prefix(word).is_some_and(starts_with_boundary))
}
fn starts_with_boundary(rest: &str) -> bool {
    is_boundary(rest.chars().next())
}
fn has_unnegated_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let after_index = absolute_index + phrase.len();
        if phrase_has_boundaries(text, absolute_index, after_index) {
            let prefix_start = char_window_start(text, absolute_index, negation_window);
            if !has_nearby_negation(&text[prefix_start..absolute_index]) {
                return true;
            }
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}
fn has_unnegated_word(text: &str, word: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(word) {
        let absolute_index = offset + index;
        let after_index = absolute_index + word.len();
        if is_boundary(text[..absolute_index].chars().next_back())
            && is_boundary(text[after_index..].chars().next())
        {
            let prefix_start = char_window_start(text, absolute_index, negation_window);
            if !has_nearby_negation(&text[prefix_start..absolute_index]) {
                return true;
            }
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}
fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}
fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}
fn has_nearby_negation(prefix: &str) -> bool {
    "no|no user or|no explicit|not|not yet|not explicit|isn't|isn't yet|aren't yet|is not|did not|did not explicitly|was not|was not explicitly|were not|were not explicitly|without|without explicit|neither"
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
fn pr_number(pr_state: &Value) -> String {
    pr_state
        .get("number")
        .and_then(Value::as_u64)
        .map_or_else(|| "<unknown>".to_owned(), |number| number.to_string())
}
fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
