use serde_json::Value;

pub(super) fn check(handoff: &str, pr_state: &str) -> Vec<String> {
    let pr_state = match serde_json::from_str::<Value>(pr_state) {
        Ok(value) => value,
        Err(error) => {
            return vec![format!(
                "completion handoff PR state must be valid JSON: {error}"
            )];
        }
    };

    if !is_clean_open_pr(&pr_state)
        || !claims_completion(handoff)
        || states_explicit_deferral(handoff)
    {
        return Vec::new();
    }

    vec![format!(
        "opening a PR is not completion: PR #{} is still open and mergeable; state an explicit stop, wait, draft-only, leave-open, or no-merge deferral instead of claiming completion",
        pr_number(&pr_state)
    )]
}

fn is_clean_open_pr(pr_state: &Value) -> bool {
    string_field(pr_state, "state").is_some_and(|state| state.eq_ignore_ascii_case("OPEN"))
        && !bool_field(pr_state, "isDraft").unwrap_or(false)
        && string_field(pr_state, "mergeStateStatus").is_some_and(|status| {
            matches!(status.to_ascii_uppercase().as_str(), "CLEAN" | "HAS_HOOKS")
        })
}

fn claims_completion(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    [
        "work is complete",
        "task is complete",
        "goal is complete",
        "goal complete",
        "lane is complete",
        "implementation is complete",
        "complete after opening pr",
        "complete after pr",
        "complete.",
        "complete\n",
        "successfully completed",
        "completed",
        "finished",
        "finalized",
        "all set",
        "done.",
        "done\n",
        "done after opening pr",
        "done after pr",
        "is done",
    ]
    .iter()
    .any(|phrase| has_unnegated_phrase(&text, phrase, 16))
        || has_unnegated_word(&text, "done", 16)
        || has_unnegated_word(&text, "complete", 16)
        || has_unnegated_word(&text, "finish", 16)
        || has_unnegated_word(&text, "finalize", 16)
}

fn states_explicit_deferral(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();

    [
        "per the stop condition",
        "maintainer requested stop",
        "maintainer requested wait",
        "maintainer requested no merge",
        "maintainer requested leave open",
        "maintainer explicitly requested stop",
        "maintainer explicitly requested wait",
        "maintainer explicitly requested no merge",
        "maintainer explicitly requested leave open",
        "maintainer asked me to stop",
        "maintainer asked me to wait",
        "maintainer asked me to leave open",
        "do not merge per maintainer",
        "no merge per maintainer",
        "no-merge instruction",
        "maintainer requested draft-only",
        "maintainer explicitly requested draft-only",
        "draft pr per maintainer",
        "draft pull request per maintainer",
        "draft-only instruction",
        "leave open per maintainer",
        "left open per maintainer",
        "deferred by maintainer",
    ]
    .iter()
    .any(|phrase| has_unnegated_deferral_phrase(&text, phrase, 80))
}

fn has_unnegated_deferral_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let after_index = absolute_index + phrase.len();
        if phrase_has_boundaries(text, absolute_index, after_index)
            && !has_false_deferral_label_after(text, after_index)
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

fn has_false_deferral_label_after(text: &str, after_index: usize) -> bool {
    let suffix = text[after_index..].trim_start();
    let Some(value) = suffix.strip_prefix(':') else {
        return false;
    };
    let value = value.trim_start();
    if matches!(value.chars().next(), None | Some('.') | Some(';')) {
        return true;
    }
    ["none", "false", "not requested", "no"].iter().any(|word| {
        value
            .strip_prefix(word)
            .is_some_and(|rest| is_boundary(rest.chars().next()))
    })
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
    let prefix = prefix.trim_end();
    [
        "no",
        "no explicit",
        "not",
        "not explicit",
        "isn't",
        "is not",
        "did not",
        "did not explicitly",
        "was not",
        "was not explicitly",
        "were not",
        "were not explicitly",
        "without",
        "without explicit",
        "neither",
    ]
    .iter()
    .any(|phrase| prefix.ends_with(phrase))
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

fn bool_field(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}
