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
        "successfully completed",
        "completed",
        "all set",
        "done.",
        "done\n",
        "done after opening pr",
        "done after pr",
        "is done",
    ]
    .iter()
    .any(|phrase| has_unnegated_phrase(&text, phrase, 16))
}

fn states_explicit_deferral(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();

    [
        "per the stop condition",
        "stop condition:",
        "maintainer requested stop",
        "maintainer requested wait",
        "maintainer requested no merge",
        "maintainer requested leave open",
        "maintainer explicitly requested stop",
        "maintainer explicitly requested wait",
        "maintainer explicitly requested no merge",
        "maintainer explicitly requested leave open",
        "asked me to stop",
        "asked me to wait",
        "asked me to leave open",
        "do not merge per maintainer",
        "no merge per maintainer",
        "no-merge instruction",
        "maintainer requested draft-only",
        "maintainer explicitly requested draft-only",
        "asked me to leave open",
        "draft pr per maintainer",
        "draft pull request per maintainer",
        "draft-only instruction",
        "leave open per maintainer",
        "left open per maintainer",
        "deferred by maintainer",
    ]
    .iter()
    .any(|phrase| has_unnegated_phrase(&text, phrase, 80))
}

fn has_unnegated_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let prefix_start = absolute_index.saturating_sub(negation_window);
        if !has_nearby_negation(&text[prefix_start..absolute_index]) {
            return true;
        }
        offset = absolute_index + phrase.len();
        rest = &text[offset..];
    }
    false
}

fn has_nearby_negation(prefix: &str) -> bool {
    [
        "no ",
        " no ",
        " no-",
        "not ",
        " not ",
        "did not ",
        " did not ",
        "was not ",
        " was not ",
        "were not ",
        " were not ",
        "without ",
        " without ",
        "neither ",
        " neither ",
    ]
    .iter()
    .any(|phrase| prefix.contains(phrase))
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
