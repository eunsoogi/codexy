mod parser;

use std::collections::BTreeSet;

use parser::{
    OrderedEvent, active_terminal_reviewer_result_lines, field, has_distinct_values,
    has_elapsed_minimum, is_blocked_pre_delivery, is_terminal_goal_call, normalized_lines,
    ordered_event,
};

const NONTERMINAL_PRODUCERS: &[&str] = &[
    "sentinel-running",
    "child-pending",
    "ci-queued",
    "connector-review-pending",
];

pub(super) fn check(evidence: &str) -> Vec<String> {
    let lines = normalized_lines(evidence);
    let terminal_result_lines = active_terminal_reviewer_result_lines(evidence);
    let mut errors = Vec::new();
    let mut child_owned = false;
    let mut lane_start = 0;
    for (index, line) in lines.iter().enumerate() {
        if is_lane_boundary(line) {
            if child_owned {
                errors.extend(check_lane(
                    &lines[lane_start..index],
                    &terminal_result_lines,
                    lane_start,
                ));
            }
            child_owned = is_child_boundary(line);
            lane_start = index;
        }
    }
    if child_owned {
        errors.extend(check_lane(
            &lines[lane_start..],
            &terminal_result_lines,
            lane_start,
        ));
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

fn check_lane(
    lines: &[String],
    terminal_result_lines: &BTreeSet<usize>,
    offset: usize,
) -> Vec<String> {
    let mut errors = check_wait_handoffs(lines, terminal_result_lines, offset);
    for (call_index, line) in lines.iter().enumerate() {
        if ordered_event(line) == OrderedEvent::BlockedCall {
            errors.extend(check_blocked_call(lines, call_index));
        }
    }
    errors
}

fn check_blocked_call(lines: &[String], call_index: usize) -> Vec<String> {
    let mut errors = Vec::new();
    let pre_delivery_index = lines[..call_index]
        .iter()
        .rposition(|line| is_blocked_pre_delivery(line));
    let Some(pre_delivery_index) = pre_delivery_index else {
        return vec![
            "blocked goal call requires a typed blocked goal audit before its pre-delivery receipt"
                .into(),
        ];
    };
    let audit_index = lines[..pre_delivery_index]
        .iter()
        .rposition(|line| line.starts_with("blocked goal audit:"));
    let Some(audit_index) = audit_index else {
        return vec!["blocked goal call requires a typed blocked goal audit".into()];
    };
    let audit = &lines[audit_index];
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
    let pre_mutation_index = lines[pre_delivery_index + 1..call_index]
        .iter()
        .rposition(|line| line.starts_with("blocked goal pre-mutation check:"))
        .map(|index| pre_delivery_index + 1 + index);
    let pre_mutation = pre_mutation_index.map(|index| &lines[index]);
    let parent_direction_in_window = lines[audit_index + 1..call_index]
        .iter()
        .any(|line| ordered_event(line) == OrderedEvent::ParentDirection);
    let delivered_version = field(&lines[pre_delivery_index], "parent direction version");
    if parent_direction_in_window || !valid_pre_mutation(pre_mutation, audit_id, delivered_version)
    {
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

fn check_wait_handoffs(
    lines: &[String],
    terminal_result_lines: &BTreeSet<usize>,
    offset: usize,
) -> Vec<String> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("nonterminal wait handoff:"))
        .filter_map(|(index, line)| {
            (invalid_field(field(line, "state fingerprint"))
                || !field(line, "producer state")
                    .is_some_and(|value| NONTERMINAL_PRODUCERS.contains(&value))
                || invalid_wake_route(field(line, "wake route"))
                || field(line, "ownership") != Some("retained")
                || field(line, "goal state") != Some("active")
                || field(line, "plan state") != Some("active")
                || field(line, "goal transition") != Some("none")
                || field(line, "return control") != Some("confirmed")
                || terminal_goal_precedes_reviewer_result(
                    &lines[index + 1..],
                    terminal_result_lines,
                    offset + index + 1,
                ))
            .then_some("nonterminal wait handoff requires a stable fingerprint, nonterminal producer, available wake route, retained ownership, active goal and plan state, no complete/blocked goal mutation before a terminal reviewer result, and confirmed return control".into())
        })
        .collect()
}

fn terminal_goal_precedes_reviewer_result(
    lines: &[String],
    terminal_result_lines: &BTreeSet<usize>,
    offset: usize,
) -> bool {
    lines
        .iter()
        .enumerate()
        .take_while(|(index, _)| !terminal_result_lines.contains(&(offset + index)))
        .any(|(_, line)| is_terminal_goal_call(line))
}

fn invalid_field(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.is_empty() || matches!(value, "none" | "unavailable"))
}

fn invalid_wake_route(value: Option<&str>) -> bool {
    invalid_field(value)
}
