use std::collections::BTreeSet;

const NONTERMINAL_PRODUCERS: &[&str] = &[
    "sentinel-running",
    "child-pending",
    "ci-queued",
    "connector-review-pending",
];

pub(super) fn check(evidence: &str) -> Vec<String> {
    let lines = evidence
        .lines()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .map(|line| without_list_prefix(&line).to_owned())
        .collect::<Vec<_>>();
    let mut errors = Vec::new();
    let mut child_owned = false;
    let mut lane_start = 0;
    for (index, line) in lines.iter().enumerate() {
        if is_lane_boundary(line) {
            if child_owned {
                errors.extend(check_lane(&lines[lane_start..index]));
            }
            child_owned = is_child_boundary(line);
            lane_start = index;
        }
    }
    if child_owned {
        errors.extend(check_lane(&lines[lane_start..]));
    }
    errors
}

fn is_lane_boundary(line: &str) -> bool {
    line.starts_with("lane ownership:") || line.starts_with("owner decision:")
}

fn is_child_boundary(line: &str) -> bool {
    line.contains("lane ownership: child-owned")
        || line.starts_with("owner decision: affirmative child-owned")
}

fn check_lane(lines: &[String]) -> Vec<String> {
    let mut errors = check_wait_handoffs(lines);
    for (call_index, line) in lines.iter().enumerate() {
        if line.starts_with("goal tool call: update_goal(blocked)") {
            errors.extend(check_blocked_call(lines, call_index));
        }
    }
    errors
}

fn without_list_prefix(line: &str) -> &str {
    let numbered = line.trim_start_matches(|character: char| character.is_ascii_digit());
    numbered
        .strip_prefix(". ")
        .or_else(|| numbered.strip_prefix(") "))
        .or_else(|| line.strip_prefix("- "))
        .or_else(|| line.strip_prefix("* "))
        .unwrap_or(line)
}

fn check_blocked_call(lines: &[String], call_index: usize) -> Vec<String> {
    let mut errors = Vec::new();
    let pre_delivery_index = lines[..call_index].iter().rposition(|line| {
        line.starts_with("parent goal pre-delivery: operation=update_goal(blocked)")
    });
    let Some(pre_delivery_index) = pre_delivery_index else {
        return vec![
            "blocked goal call requires a typed blocked goal audit before its pre-delivery receipt"
                .into(),
        ];
    };
    let audit = lines[..pre_delivery_index]
        .iter()
        .rfind(|line| line.starts_with("blocked goal audit:"));
    let Some(audit) = audit else {
        return vec!["blocked goal call requires a typed blocked goal audit".into()];
    };
    let audit_id = field(audit, "audit id");
    if audit_id.is_none_or(str::is_empty) {
        errors.push("blocked goal audit requires an audit id".into());
    }
    if !has_distinct_values(audit, "observation ids", 3)
        || !has_distinct_values(audit, "state fingerprints", 3)
    {
        errors.push(
            "blocked goal audit requires three distinct material observations and fingerprints"
                .into(),
        );
    }
    if !has_elapsed_minimum(audit) {
        errors.push("blocked goal audit requires monotonic elapsed time at least the positive declared minimum".into());
    }
    match field(audit, "producer state") {
        Some("none" | "terminal-failure") => {}
        Some(value) if NONTERMINAL_PRODUCERS.contains(&value) => {
            errors.push("blocked goal audit has an active external producer".into());
        }
        _ => errors
            .push("blocked goal audit requires producer state none or terminal-failure".into()),
    }
    if field(audit, "safe action") != Some("unavailable") {
        errors.push("blocked goal audit requires safe action=unavailable".into());
    }
    if field(audit, "wake route") != Some("unavailable") {
        errors.push("blocked goal audit requires wake route=unavailable".into());
    }
    let pre_mutation = lines[pre_delivery_index + 1..call_index]
        .iter()
        .rfind(|line| line.starts_with("blocked goal pre-mutation check:"));
    let delivered_version = field(&lines[pre_delivery_index], "parent direction version");
    if !valid_pre_mutation(pre_mutation, audit_id, delivered_version) {
        errors.push("blocked goal call is cancelled by newer parent direction or lacks a final matching pre-mutation check".into());
        if delivered_version.is_none() {
            errors.push(
                "blocked goal pre-delivery receipt requires a parent direction version".into(),
            );
        } else {
            errors.push(
                "blocked goal pre-mutation check must match the pre-delivery parent direction version"
                    .into(),
            );
        }
    }
    errors
}

fn valid_pre_mutation(
    line: Option<&String>,
    audit_id: Option<&str>,
    delivered_version: Option<&str>,
) -> bool {
    let Some(line) = line else {
        return false;
    };
    field(line, "audit id") == audit_id
        && delivered_version
            .zip(field(line, "pre-delivery parent direction version"))
            .zip(field(line, "current parent direction version"))
            .is_some_and(|((delivered, before), current)| {
                delivered == before && before == current && !before.is_empty()
            })
        && field(line, "cancellation") == Some("absent")
}

fn check_wait_handoffs(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| line.starts_with("nonterminal wait handoff:"))
        .filter_map(|line| {
            (invalid_field(field(line, "state fingerprint"))
                || !field(line, "producer state")
                    .is_some_and(|value| NONTERMINAL_PRODUCERS.contains(&value))
                || invalid_wake_route(field(line, "wake route"))
                || field(line, "ownership") != Some("retained")
                || field(line, "goal transition") != Some("none")
                || field(line, "return control") != Some("confirmed"))
            .then_some("nonterminal wait handoff requires a stable fingerprint, nonterminal producer, available wake route, retained ownership, and no complete/blocked goal mutation".into())
        })
        .collect()
}

fn invalid_field(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.is_empty() || matches!(value, "none" | "unavailable"))
}

fn invalid_wake_route(value: Option<&str>) -> bool {
    invalid_field(value)
}

fn has_distinct_values(line: &str, name: &str, minimum: usize) -> bool {
    field(line, name)
        .map(|value| {
            value
                .split('|')
                .filter(|value| !value.is_empty())
                .collect::<BTreeSet<_>>()
                .len()
                >= minimum
        })
        .unwrap_or(false)
}

fn has_elapsed_minimum(line: &str) -> bool {
    let parse = |name| field(line, name).and_then(|value| value.parse::<u64>().ok());
    parse("first monotonic ms")
        .zip(parse("observed monotonic ms"))
        .zip(parse("minimum interval ms"))
        .is_some_and(|((first, observed), minimum)| {
            minimum > 0 && observed.saturating_sub(first) >= minimum
        })
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    line.split_once(": ")
        .map_or(line, |(_, value)| value)
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&prefix))
}
