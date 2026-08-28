mod negation;
mod parser;
pub(super) mod wait_taxonomy;

use parser::{
    ActiveEvent, OrderedEvent, active_events, field, has_distinct_substantive_values,
    is_blocked_pre_delivery, is_substantive,
};
use wait_taxonomy::{WaitDisposition, classify_producer};

pub(super) fn check(plugin_root: &std::path::Path, evidence: &str) -> Vec<String> {
    let events = active_events(plugin_root, evidence);
    let mut errors = check_wait_handoffs(&events);
    for (call_index, event) in events.iter().enumerate() {
        if event.kind == OrderedEvent::BlockedCall {
            errors.extend(check_blocked_call(&events, call_index));
        }
    }
    errors
}

fn check_blocked_call(events: &[ActiveEvent], call_index: usize) -> Vec<String> {
    let mut errors = Vec::new();
    let pre_delivery_index = events[..call_index]
        .iter()
        .rposition(|event| is_blocked_pre_delivery(&event.line));
    let Some(pre_delivery_index) = pre_delivery_index else {
        return vec![
            "blocked goal call requires a typed unanswered user-decision gate before its pre-delivery receipt".into(),
        ];
    };
    let gate_index = events[..pre_delivery_index]
        .iter()
        .rposition(|event| event.line.starts_with("blocked goal user-decision gate:"));
    let Some(gate_index) = gate_index else {
        return vec!["blocked goal call requires a typed unanswered user-decision gate".into()];
    };
    let gate = &events[gate_index].line;
    let gate_id = field(gate, "gate id");
    if invalid_field(gate_id) {
        errors.push("blocked goal user-decision gate requires a gate id".into());
    }
    if !matches!(
        field(gate, "blocker class"),
        Some("user-decision" | "missing-user-information")
    ) {
        errors.push("blocked goal gate requires a user-decision blocker class".into());
    }
    if field(gate, "decision owner") != Some("user") {
        errors.push("blocked goal gate requires decision owner=user".into());
    }
    if field(gate, "user response") != Some("unanswered")
        || invalid_question(field(gate, "user question"))
    {
        errors.push("blocked goal gate requires an exact unanswered user question".into());
    }
    if !has_distinct_substantive_values(gate, "decision branches", 2, 3, 12, 2)
        || !field(gate, "material impact").is_some_and(|value| is_substantive(value, 4, 16, 2))
    {
        errors.push("blocked goal gate requires distinct material decision branches".into());
    }
    if field(gate, "safe default") != Some("unavailable")
        || field(gate, "in-scope action") != Some("unavailable")
    {
        errors.push("blocked goal gate requires no safe default or in-scope action".into());
    }
    let pre_mutation_index = events[pre_delivery_index + 1..call_index]
        .iter()
        .rposition(|event| event.line.starts_with("blocked goal pre-mutation check:"))
        .map(|index| pre_delivery_index + 1 + index);
    let pre_mutation = pre_mutation_index.map(|index| &events[index].line);
    let parent_direction_in_window = events[gate_index + 1..call_index]
        .iter()
        .any(|event| event.kind == OrderedEvent::ParentDirection);
    let delivered_version = field(&events[pre_delivery_index].line, "parent direction version");
    if parent_direction_in_window || !valid_pre_mutation(pre_mutation, gate_id, delivered_version) {
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
    gate_id: Option<&str>,
    delivered_version: Option<&str>,
) -> bool {
    let Some(line) = line else {
        return false;
    };
    field(line, "gate id") == gate_id
        && delivered_version
            .zip(field(line, "pre-delivery parent direction version"))
            .zip(field(line, "current parent direction version"))
            .is_some_and(|((delivered, before), current)| {
                delivered == before && before == current && !before.is_empty()
            })
        && field(line, "cancellation") == Some("absent")
}

fn check_wait_handoffs(events: &[ActiveEvent]) -> Vec<String> {
    events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.line.starts_with("nonterminal wait handoff:"))
        .filter_map(|(index, event)| {
            (invalid_field(field(&event.line, "state fingerprint"))
                || !field(&event.line, "producer state")
                    .and_then(classify_producer)
                    .is_some_and(|state| state == WaitDisposition::Nonterminal)
                || invalid_wake_route(field(&event.line, "wake route"))
                || field(&event.line, "ownership") != Some("retained")
                || field(&event.line, "goal state") != Some("active")
                || field(&event.line, "plan state") != Some("active")
                || field(&event.line, "goal transition") != Some("none")
                || field(&event.line, "return control") != Some("confirmed")
                || terminal_goal_precedes_typed_review_terminal(&events[index + 1..])
            )
            .then_some("nonterminal wait handoff requires a stable fingerprint, nonterminal producer, available wake route, retained ownership, active goal and plan state, no complete/blocked goal mutation before typed profile-routed terminal review evidence, and confirmed return control".into())
        })
        .collect()
}

fn terminal_goal_precedes_typed_review_terminal(events: &[ActiveEvent]) -> bool {
    events
        .iter()
        .take_while(|event| event.kind != OrderedEvent::TypedReviewTerminal)
        .any(|event| {
            matches!(
                event.kind,
                OrderedEvent::BlockedCall | OrderedEvent::TerminalGoalCall
            )
        })
}

fn invalid_field(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.is_empty() || matches!(value, "none" | "unavailable"))
}

fn invalid_question(value: Option<&str>) -> bool {
    invalid_field(value)
        || value.is_none_or(|value| {
            !value.ends_with('?') || !is_substantive(value.trim_end_matches('?'), 4, 12, 2)
        })
}

fn invalid_wake_route(value: Option<&str>) -> bool {
    invalid_field(value)
}
