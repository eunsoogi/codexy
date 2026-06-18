pub(super) fn has_negative_field_value(line: &str, field: &str) -> bool {
    field_value(line, field).is_some_and(|value| has_absent_field_value(value, field))
}

pub(super) fn next_line_bullet_value<'a>(lines: &'a [&str], index: usize) -> Option<&'a str> {
    lines
        .iter()
        .skip(index + 1)
        .find(|line| !line.is_empty())
        .map(|value| value.trim_start_matches(['-', '*']).trim())
}

pub(super) fn has_passive_parent_fix(line: &str) -> bool {
    let authored = parent_verification_prefix(line);
    (authored.contains(" by parent")
        || authored.contains(" by the parent")
        || authored.contains(" by orchestrator")
        || authored.contains(" by the orchestrator"))
        && has_fix_marker(authored)
}

fn parent_verification_prefix(line: &str) -> &str {
    ["; verified by ", ", verified by ", " and verified by "]
        .into_iter()
        .filter_map(|marker| line.find(marker).map(|index| &line[..index]))
        .min_by_key(|prefix| prefix.len())
        .unwrap_or(line)
        .trim_start()
}

pub(super) fn has_fix_marker(line: &str) -> bool {
    ["review-response", "review response", "fix", "commit"]
        .into_iter()
        .any(|marker| line.contains(marker))
}

pub(super) fn has_affirmative_implementation_field(line: &str) -> bool {
    field_value(line, "parent-authored implementation")
        .or_else(|| field_value(line, "parent authored implementation"))
        .or_else(|| field_value(line, "orchestrator-authored implementation"))
        .or_else(|| field_value(line, "orchestrator authored implementation"))
        .is_some_and(|value| matches!(trimmed_value(value), "yes" | "true"))
}

pub(super) fn has_draft_handoff_phrase(line: &str, marker: &str) -> bool {
    line.find(marker).is_some_and(|index| {
        let after_draft = &line[index..];
        after_draft.contains("draft")
            && (after_draft.contains("handoff") || after_draft.contains("routed"))
            && !after_draft.contains(&format!("{marker} implementation commit"))
            && !after_draft.contains(&format!("{marker} commit"))
            && !after_draft.contains(&format!("{marker} review-response"))
            && !after_draft.contains(&format!("{marker} review response"))
    })
}

pub(super) fn field_value<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    line.split_once(':')
        .and_then(|(key, value)| metadata_key(key).contains(field).then_some(value.trim()))
}

pub(super) fn has_non_affirmative_reassignment_key(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        let key = metadata_key(key);
        key.contains("maintainer reassignment")
            && [
                "no",
                "not",
                "without",
                "missing",
                "absent",
                "pending",
                "requested",
                "needed",
                "required",
                "denied",
                "rejected",
            ]
            .into_iter()
            .any(|qualifier| key.contains(qualifier))
    })
}

pub(super) fn metadata_key(key: &str) -> &str {
    key.trim().trim_start_matches(['-', '*']).trim_start()
}

pub(super) fn is_positive_reassignment_value(value: &str) -> bool {
    "explicit maintainer reassignment to parent|explicit maintainer reassignment to the parent|explicit maintainer reassignment to orchestrator|explicit maintainer reassignment to the orchestrator|explicit reassignment to parent|explicit reassignment to the parent|explicit reassignment to orchestrator|explicit reassignment to the orchestrator|reassigned to parent|reassigned to the parent|reassigned to orchestrator|reassigned to the orchestrator|reassigns implementation ownership to parent|reassigns implementation ownership to the parent|reassigns implementation ownership to orchestrator|reassigns implementation ownership to the orchestrator|reassigned implementation ownership to parent|reassigned implementation ownership to the parent|reassigned implementation ownership to orchestrator|reassigned implementation ownership to the orchestrator"
        .split('|')
        .any(|marker| value.contains(marker))
}

pub(super) fn is_negative_reassignment_value(value: &str) -> bool {
    let value = trimmed_value(value);
    has_absent_value(value)
        || value.starts_with("no ")
        || value.starts_with("missing ")
        || value.starts_with("absent ")
        || ["pending ", "requested ", "needed ", "required "]
            .into_iter()
            .any(|prefix| value.starts_with(prefix))
        || value.starts_with("not ")
        || value.starts_with("without ")
        || value.starts_with("[ ]")
        || ["does not include ", "does not have "]
            .into_iter()
            .any(|marker| value.contains(marker))
        || value.contains(" not reassigned to ")
        || value.contains(" not reassigned implementation ownership to ")
        || value.starts_with("we need ")
        || value.starts_with("waiting for ")
        || value.starts_with("there is no ")
        || value.starts_with("there was no ")
        || value.contains(" not provided")
        || value.ends_with(" is missing")
        || value.contains(" not granted")
        || value.contains(" was not granted")
        || value.contains(" not been granted")
        || [" denied", " rejected"]
            .into_iter()
            .any(|marker| value.contains(marker))
        || [" requested", " needed", " required", " pending"]
            .into_iter()
            .any(|marker| value.split_once(marker).is_some_and(is_empty_or_spaced))
}

fn is_empty_or_spaced((_, suffix): (&str, &str)) -> bool {
    suffix.is_empty() || suffix.starts_with(char::is_whitespace)
}

pub(super) fn has_absent_value(value: &str) -> bool {
    let value = trimmed_value(value);
    matches!(
        value,
        "no" | "none" | "false" | "missing" | "absent" | "not provided"
    )
}

pub(super) fn has_absent_field_value(value: &str, field: &str) -> bool {
    let value = trimmed_value(value);
    if has_absent_value(value) {
        return true;
    }
    if value
        .strip_prefix("not ")
        .and_then(|suffix| suffix.strip_prefix(field))
        .is_some_and(|suffix| {
            suffix.is_empty()
                || suffix.starts_with(char::is_whitespace)
                || matches!(suffix.chars().next(), Some('.' | ',' | ';' | ':'))
        })
    {
        return true;
    }
    "not provided|without|missing|absent|none|not|no"
        .split('|')
        .any(|marker| {
            let Some(after_marker) = value.strip_prefix(marker) else {
                return false;
            };
            let Some(separator) = after_marker.chars().next() else {
                return true;
            };
            if !separator.is_ascii_whitespace() && !matches!(separator, '.' | ',' | ';' | ':') {
                return false;
            }
            !after_marker.contains(field)
        })
}

pub(super) fn has_absent_authored_phrase(line: &str, marker: &str) -> bool {
    ["no ", "not ", "without "]
        .into_iter()
        .map(|prefix| format!("{prefix}{marker}"))
        .any(|absent_marker| {
            let Some(index) = line.find(&absent_marker) else {
                return false;
            };
            let after_absence = &line[index + absent_marker.len()..];
            !after_absence.contains(marker)
        })
}

pub(super) fn has_nested_absent_authored_field(line: &str, marker: &str) -> bool {
    [
        "implementation commits",
        "implementation commit",
        "implementation",
        "review-response commits",
        "review-response commit",
        "review response commits",
        "review response commit",
    ]
    .into_iter()
    .map(|field| format!("{marker} {field}:"))
    .any(|field| {
        let Some((_, value)) = line.split_once(&field) else {
            return false;
        };
        let absent_value = value.split(';').next().unwrap_or(value);
        if !has_absent_field_value(absent_value, marker) {
            return false;
        }
        value
            .split_once(';')
            .map(|(_, suffix)| !suffix.contains(marker))
            .unwrap_or(true)
    })
}

pub(super) fn has_absent_actor_phrase(line: &str, actor: &str, marker: &str) -> bool {
    ["no ", "not ", "without "]
        .into_iter()
        .map(|prefix| format!("{prefix}{actor} {marker}"))
        .any(|absent_marker| {
            let Some(index) = line.find(&absent_marker) else {
                return false;
            };
            let after_absence = &line[index + absent_marker.len()..];
            !after_absence.contains(&format!("{actor} {marker}"))
        })
}

pub(super) fn trimmed_value(value: &str) -> &str {
    value.trim_matches(|character: char| {
        character.is_ascii_whitespace() || matches!(character, '.' | ',' | ';')
    })
}
