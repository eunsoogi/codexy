pub(super) fn has_placeholder_or_pending_value(clause: &str) -> bool {
    clause.split_once(':').map_or_else(
        || starts_with_absent_issue_value(strip_issue_field_prefix(clause)),
        |(field, value)| {
            starts_with_absent_issue_value(strip_issue_field_prefix(field))
                || starts_with_absent_issue_value(value.trim())
        },
    )
}

fn strip_issue_field_prefix(clause: &str) -> &str {
    const ISSUE_FIELD_PREFIXES: &str = "separate dogfood issue|separate dogfooding issue|separate tracking issue|tracking issue|tracked in issue|tracked by issue|follow-up issue";
    let clause = strip_list_marker(clause);
    ISSUE_FIELD_PREFIXES
        .split('|')
        .find_map(|prefix| {
            clause.strip_prefix(prefix).map(|value| {
                value.trim_start_matches(|character: char| {
                    character.is_ascii_whitespace()
                        || matches!(character, ':' | '=' | '-' | '\u{2013}' | '\u{2014}')
                })
            })
        })
        .unwrap_or(clause)
}

fn strip_list_marker(clause: &str) -> &str {
    let clause = clause.trim_start();
    if let Some(value) = clause
        .strip_prefix("- ")
        .or_else(|| clause.strip_prefix("* "))
    {
        return value.trim_start();
    }
    let Some((marker, value)) = clause.split_once(['.', ')']) else {
        return clause;
    };
    if !marker.is_empty() && marker.chars().all(|character| character.is_ascii_digit()) {
        return value.trim_start();
    }
    clause
}

fn starts_with_absent_issue_value(value: &str) -> bool {
    const PLACEHOLDER_PREFIXES: &str =
        "none|n/a|tbd|pending|missing|absent|unavailable|no issue|no separate issue";
    const PENDING_PREFIXES: &str = "not created|not available|not provided|not yet created|not yet filed|will be|to be created|to be filed|planned";
    PLACEHOLDER_PREFIXES
        .split('|')
        .chain(PENDING_PREFIXES.split('|'))
        .any(|placeholder| {
            if placeholder == "missing"
                && (value.starts_with("missing-handler") || value.starts_with("missing handler"))
            {
                return false;
            }
            value == placeholder
                || value.strip_prefix(placeholder).is_some_and(|rest| {
                    rest.starts_with(|character: char| !character.is_ascii_alphanumeric())
                })
        })
        || starts_with_absent_issue_lifecycle(value)
}

fn starts_with_absent_issue_lifecycle(value: &str) -> bool {
    let value = value
        .trim_start_matches(|character: char| {
            character.is_ascii_whitespace()
                || matches!(character, ':' | '=' | '-' | '\u{2013}' | '\u{2014}')
        })
        .replace("isn't", "is not")
        .replace("wasn't", "was not")
        .replace("hasn't", "has not")
        .replace("won't", "will not");
    ["created", "filed", "opened", "provided", "available"]
        .into_iter()
        .any(|state| {
            [
                format!("not {state}"),
                format!("not yet {state}"),
                format!("not {state} yet"),
                format!("not been {state}"),
                format!("not yet been {state}"),
                format!("not been {state} yet"),
                format!("will be {state}"),
                format!("will not be {state}"),
                format!("to be {state}"),
                format!("should be {state}"),
                format!("needs to be {state}"),
            ]
            .into_iter()
            .any(|prefix| has_absent_prefix(&value, &prefix))
        })
        || ["planned", "pending"]
            .into_iter()
            .any(|prefix| has_absent_prefix(&value, prefix))
}

fn has_absent_prefix(value: &str, prefix: &str) -> bool {
    value == prefix
        || value.strip_prefix(prefix).is_some_and(|rest| {
            rest.starts_with(|character: char| !character.is_ascii_alphanumeric())
        })
}
