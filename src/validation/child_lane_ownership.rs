pub(super) fn check(evidence: &str) -> Vec<String> {
    let normalized = evidence.to_lowercase();
    if !has_affirmative_child_owned_lane(&normalized)
        || has_explicit_maintainer_reassignment(&normalized)
        || !has_parent_authored_fix(&normalized)
    {
        return Vec::new();
    }

    vec![
        "child-owned lane contains parent-authored implementation or review-response evidence without explicit maintainer reassignment".to_owned(),
    ]
}
fn has_affirmative_child_owned_lane(evidence: &str) -> bool {
    evidence.lines().map(str::trim).any(|line| {
        field_value(line, "ownership").is_some_and(is_affirmative_child_owned_value)
            || field_value(line, "child-owned lane")
                .is_some_and(|value| matches!(trimmed_value(value), "yes" | "true" | "child-owned"))
            || matches!(trimmed_value(line), "child-owned" | "child-owned lane")
    })
}
fn is_affirmative_child_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    value.contains("child-owned")
        && !value.contains("not child-owned")
        && !value.contains("parent-owned")
        && !has_absent_field_value(value, "child-owned")
}
fn has_explicit_maintainer_reassignment(evidence: &str) -> bool {
    let lines = evidence.lines().map(str::trim).collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        let line = line.trim();
        if has_non_affirmative_reassignment_key(line) {
            return false;
        }
        let Some(value) = field_value(line, "maintainer reassignment") else {
            return false;
        };
        let value = if value.is_empty() {
            next_line_bullet_value(&lines, index).unwrap_or(value)
        } else {
            value
        };
        is_positive_reassignment_value(value) && !is_negative_reassignment_value(value)
    })
}
fn has_parent_authored_fix(evidence: &str) -> bool {
    let lines = evidence.lines().map(str::trim).collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        if has_empty_field_value(line, "parent-authored")
            && next_line_has_absent_value(&lines, index)
        {
            return false;
        }
        if line.contains("parent-authored")
            && !has_negative_field_value(line, "parent-authored")
            && !has_absent_authored_phrase(line, "parent-authored")
            && !has_draft_handoff_phrase(line, "parent-authored")
        {
            return has_fix_marker(line);
        }
        if line.contains("parent authored")
            && !has_negative_field_value(line, "parent")
            && !has_absent_actor_phrase(line, "parent", "authored")
        {
            return has_fix_marker(line);
        }
        if line.contains("orchestrator-authored")
            && !has_negative_field_value(line, "orchestrator-authored")
            && !has_absent_authored_phrase(line, "orchestrator-authored")
            && !has_draft_handoff_phrase(line, "orchestrator-authored")
        {
            return has_fix_marker(line);
        }
        (line.contains("parent implemented")
            || line.contains("parent fixed")
            || line.contains("parent pushed")
            || line.contains("parent implementation commit")
            || line.contains("fixed in parent")
            || line.contains("parent patched")
            || line.contains("orchestrator patched")
            || (line.contains("orchestrator authored")
                && has_fix_marker(line)
                && !has_absent_actor_phrase(line, "orchestrator", "authored"))
            || line.contains("orchestrator fixed")
            || line.contains("orchestrator review-response")
            || line.contains("orchestrator review response")
            || line.contains("parent review-response")
            || line.contains("parent review response")
            || (line.contains("parent commit")
                && !has_absent_actor_phrase(line, "parent", "commit"))
            || has_passive_parent_fix(line)
            || line.contains("patched by parent"))
            && !has_negative_field_value(line, "parent")
    })
}
fn has_negative_field_value(line: &str, field: &str) -> bool {
    field_value(line, field).is_some_and(|value| has_absent_field_value(value, field))
}
fn has_empty_field_value(line: &str, field: &str) -> bool {
    field_value(line, field).is_some_and(str::is_empty)
}
fn next_line_has_absent_value(lines: &[&str], index: usize) -> bool {
    let Some(value) = lines.iter().skip(index + 1).find(|line| !line.is_empty()) else {
        return false;
    };
    has_absent_value(value.trim_start_matches(['-', '*']).trim())
}
fn next_line_bullet_value<'a>(lines: &'a [&str], index: usize) -> Option<&'a str> {
    let value = lines.iter().skip(index + 1).find(|line| !line.is_empty())?;
    Some(
        value
            .strip_prefix('-')
            .or_else(|| value.strip_prefix('*'))
            .unwrap_or(value)
            .trim(),
    )
}
fn has_passive_parent_fix(line: &str) -> bool {
    (line.contains(" by parent")
        || line.contains(" by the parent")
        || line.contains(" by orchestrator")
        || line.contains(" by the orchestrator"))
        && has_fix_marker(line)
}
fn has_fix_marker(line: &str) -> bool {
    "review-response|review response|fix|commit"
        .split('|')
        .any(|marker| line.contains(marker))
}
fn has_draft_handoff_phrase(line: &str, marker: &str) -> bool {
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
fn field_value<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    line.split_once(':')
        .and_then(|(key, value)| key.contains(field).then_some(value.trim()))
}
fn has_non_affirmative_reassignment_key(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        key.contains("maintainer reassignment")
            && ["pending", "requested", "needed", "required"]
                .into_iter()
                .any(|qualifier| key.contains(qualifier))
    })
}
fn is_positive_reassignment_value(value: &str) -> bool {
    "explicit maintainer reassignment to parent|explicit maintainer reassignment to the parent|explicit maintainer reassignment to orchestrator|explicit maintainer reassignment to the orchestrator|explicit reassignment to parent|explicit reassignment to the parent|explicit reassignment to orchestrator|explicit reassignment to the orchestrator|reassigned to parent|reassigned to the parent|reassigned to orchestrator|reassigned to the orchestrator|reassigns implementation ownership to parent|reassigns implementation ownership to the parent|reassigns implementation ownership to orchestrator|reassigns implementation ownership to the orchestrator|reassigned implementation ownership to parent|reassigned implementation ownership to the parent|reassigned implementation ownership to orchestrator|reassigned implementation ownership to the orchestrator"
        .split('|')
        .any(|marker| value.contains(marker))
}
fn is_negative_reassignment_value(value: &str) -> bool {
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
        || value.starts_with("we need ")
        || value.starts_with("waiting for ")
        || value.starts_with("there is no ")
        || value.starts_with("there was no ")
        || value.contains(" not provided")
        || value.ends_with(" is missing")
        || value.contains(" not granted")
        || value.contains(" was not granted")
        || value.contains(" not been granted")
        || value.contains(" was denied")
        || value.contains(" was rejected")
        || [" requested", " needed", " required", " pending"]
            .into_iter()
            .any(|suffix| contains_non_affirmative_reassignment_suffix(value, suffix))
}
fn contains_non_affirmative_reassignment_suffix(value: &str, marker: &str) -> bool {
    value
        .split_once(marker)
        .is_some_and(|(_, suffix)| suffix.is_empty() || suffix.starts_with(char::is_whitespace))
}
fn has_absent_value(value: &str) -> bool {
    let value = trimmed_value(value);
    matches!(value, "no" | "none" | "missing" | "absent" | "not provided")
}
fn has_absent_field_value(value: &str, field: &str) -> bool {
    let value = trimmed_value(value);
    if has_absent_value(value) {
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
fn has_absent_authored_phrase(line: &str, marker: &str) -> bool {
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
fn has_absent_actor_phrase(line: &str, actor: &str, marker: &str) -> bool {
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
fn trimmed_value(value: &str) -> &str {
    value.trim_matches(|character: char| {
        character.is_ascii_whitespace() || matches!(character, '.' | ',' | ';')
    })
}
