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
    [
        " by parent",
        " by the parent",
        " by orchestrator",
        " by the orchestrator",
    ]
    .into_iter()
    .any(|marker| authored.contains(marker))
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
            && ["handoff", "routed"]
                .into_iter()
                .any(|term| after_draft.contains(term))
            && [
                "implementation commit",
                "fix commit",
                "commit",
                "review-response",
                "review response",
            ]
            .into_iter()
            .all(|suffix| !after_draft.contains(&format!("{marker} {suffix}")))
    })
}

pub(super) fn field_value<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    line.split_once(':')
        .and_then(|(key, value)| metadata_key(key).contains(field).then_some(value.trim()))
}

pub(super) fn has_non_affirmative_reassignment_key(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        let (key, unchecked_task) = metadata_prefix(key);
        key.contains("maintainer reassignment")
            && (unchecked_task
                || "no|not|without|missing|absent|pending|requested|needed|required|denied|rejected"
                    .split('|')
                .into_iter()
                .any(|qualifier| {
                    key.split(|character: char| !character.is_ascii_alphanumeric())
                        .any(|word| word == qualifier)
                }))
    })
}

pub(super) fn metadata_key(key: &str) -> &str {
    metadata_prefix(key).0
}

fn metadata_prefix(key: &str) -> (&str, bool) {
    let key = key.trim().trim_start_matches(['-', '*', '+']).trim_start();
    let unchecked_task = key.starts_with("[ ]");
    let key = key
        .strip_prefix("[ ]")
        .or_else(|| key.strip_prefix("[x]"))
        .or_else(|| key.strip_prefix("[X]"))
        .unwrap_or(key)
        .trim_start();
    let numbered = key.trim_start_matches(|character: char| character.is_ascii_digit());
    (
        numbered.strip_prefix('.').unwrap_or(key).trim_start(),
        unchecked_task,
    )
}

pub(super) fn is_metadata_field(line: &str) -> bool {
    line.split_once(':')
        .is_some_and(|(key, _)| !metadata_key(key).is_empty())
}

pub(super) fn is_positive_reassignment_value(value: &str) -> bool {
    "explicit maintainer reassignment to parent|explicit maintainer reassignment to the parent|explicit maintainer reassignment to orchestrator|explicit maintainer reassignment to the orchestrator|explicit reassignment to parent|explicit reassignment to the parent|explicit reassignment to orchestrator|explicit reassignment to the orchestrator|reassigned to parent|reassigned to the parent|reassigned to orchestrator|reassigned to the orchestrator|reassigns implementation ownership to parent|reassigns implementation ownership to the parent|reassigns implementation ownership to orchestrator|reassigns implementation ownership to the orchestrator|reassigned implementation ownership to parent|reassigned implementation ownership to the parent|reassigned implementation ownership to orchestrator|reassigned implementation ownership to the orchestrator"
        .split('|')
        .any(|marker| value.contains(marker))
}

pub(super) fn is_negative_reassignment_value(value: &str) -> bool {
    let value = trimmed_value(value);
    has_absent_value(value)
        || "no |missing |absent |pending |requested |needed |required |denied |rejected |not |without |[ ]|we need |waiting for |there is no |there was no "
            .split('|').any(|prefix| value.starts_with(prefix))
        || "does not include |does not have | not reassigned to | not reassigned implementation ownership to | not provided| is missing from | not granted| not yet granted| was not granted| not been granted"
            .split('|').any(|marker| value.contains(marker))
        || has_negated_reassignment_action(value)
        || value.ends_with(" is missing")
        || [" denied", " rejected"]
            .into_iter()
            .any(|marker| value.contains(marker))
        || [" requested", " needed", " required", " pending"]
            .into_iter()
            .any(|marker| value.split_once(marker).is_some_and(is_empty_or_spaced))
}

fn has_negated_reassignment_action(value: &str) -> bool {
    value.split_once(" not ").is_some_and(|(_, suffix)| {
        suffix.contains("reassigned to ")
            || suffix.contains("reassigned implementation ownership to ")
    })
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
