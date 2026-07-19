use std::collections::BTreeSet;

use super::child_lane_classification_boundaries::ClassificationTable;
use super::child_lane_classification_boundaries::{
    child_candidate_requires_guard, child_table_owns_handoff_pr, classification_owner_before,
    classifications, is_legacy_ownership_boundary, owner_at, table_ownership_boundary,
};
use super::child_lane_owner_decision::is_child_delegation_owner_decision;
use super::child_lane_ownership_phrases::field_value;
use super::child_terminal_handoff::{
    check as check_terminal_handoffs, is_local_task_target, is_terminal_goal_call,
    without_metadata_prefix,
};

pub(super) fn check(evidence: &str) -> Vec<String> {
    let text = evidence.to_ascii_lowercase();
    let tables = classifications(&text);
    let lines = text
        .lines()
        .map(str::trim)
        .map(without_metadata_prefix)
        .collect::<Vec<_>>();
    let mut errors = Vec::new();
    if lines.iter().enumerate().any(|(index, line)| {
        is_goal_reporting(line)
            && (child_candidate_requires_guard(&tables, &lines, index)
                || classification_owner_before(&lines, &tables, index)
                    .is_some_and(|owner| owner.starts_with("external/human-owned")))
    }) {
        errors.push("invalid table cannot authorize child goal reporting".into());
    }
    let mut start = 0;
    for end in 1..=lines.len() {
        if end == lines.len() || is_lane_boundary(&lines, &tables, end) {
            if owner_at(&tables, start).is_some_and(is_child_delegation_owner_decision)
                || lines[start].contains("lane ownership: child-owned")
                || field_value(lines[start], "owner decision")
                    .is_some_and(is_child_delegation_owner_decision)
            {
                errors.extend(check_lane(&lines[start..end]));
            }
            start = end;
        }
    }
    errors
}

fn check_lane(lines: &[&str]) -> Vec<String> {
    let has_goal_reporting = lines.iter().any(|line| is_goal_reporting(line));
    let source = lines
        .iter()
        .find_map(|line| line.strip_prefix("source thread id: "))
        .filter(|value| !value.is_empty());
    let mut errors = check_terminal_handoffs(lines, source);
    if lines.iter().any(|line| is_local_agent_route(line)) {
        errors.push("child goal reporting must not use local agents /root routing".into());
    }
    if !has_goal_reporting {
        return errors;
    }
    let Some(source) = source else {
        errors.push("child goal reporting requires source_thread_id delegation evidence".into());
        return errors;
    };
    if is_local_task_target(source) {
        errors.push("source_thread_id must name a Codex task id, not a local agent target".into());
        return errors;
    }
    let has_control_source = lines.iter().any(|line| {
        line.starts_with("goal control state:") && field(line, "source_thread_id") == Some(source)
    });
    let mut key = None;
    let mut pending = None;
    let mut confirmed_pre = None;
    let mut seen_calls = BTreeSet::new();

    for line in lines {
        if let Some(value) = line.strip_prefix("goal transition key: ") {
            key = (value.split(':').count() == 3 && value.split(':').all(|part| !part.is_empty()))
                .then_some(value);
            continue;
        }
        if let Some(operation) = event_operation(line, "parent goal pre-delivery: operation=") {
            confirmed_pre = pre_delivery_is_confirmed(line, operation, source, key, &mut errors)
                .then_some((operation, key));
            continue;
        }
        if let Some(operation) = line.strip_prefix("goal tool call: ") {
            if pending.is_some() {
                errors.push("goal operation is missing a confirmed post-result report".into());
            }
            let valid_key = key.is_some();
            if !has_control_source || !valid_key {
                errors.push("goal operation lacks a stable transition key and exact source_thread_id control state".into());
            }
            if operation == "create_goal" || is_terminal_goal_call(operation) {
                match confirmed_pre {
                    Some((pre_operation, pre_key))
                        if pre_operation == operation && pre_key == key => {}
                    Some(_) => errors.push(
                        "pre-delivery receipt does not match the goal call stable transition key"
                            .into(),
                    ),
                    None => errors.push(if operation.contains("blocked") {
                        "blocked goal operation precedes confirmed parent delivery".into()
                    } else if operation.contains("complete") {
                        "complete goal operation precedes confirmed parent delivery".into()
                    } else {
                        "goal operation requires confirmed pre-delivery parent report".into()
                    }),
                }
            }
            if let Some(value) = key.filter(|_| valid_key) {
                if !seen_calls.insert(format!("{value}:{operation}")) {
                    errors.push("duplicate goal call uses one stable transition key".into());
                }
            }
            pending = Some(operation);
            confirmed_pre = None;
            continue;
        }
        if let Some(operation) = event_operation(line, "parent goal post-result: operation=") {
            if pending == Some(operation) {
                if post_result_is_confirmed(line, operation, source, key, &mut errors) {
                    pending = None;
                }
            }
        }
    }
    if pending.is_some() {
        errors.push("goal operation is missing a confirmed post-result report".into());
    }
    errors
}
fn event_operation<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    line.strip_prefix(prefix)
        .and_then(|value| value.split(';').next())
}
fn pre_delivery_is_confirmed(
    line: &str,
    operation: &str,
    source: &str,
    key: Option<&str>,
    errors: &mut Vec<String>,
) -> bool {
    if field(line, "parent task") != Some(source) {
        errors.push("goal report names the wrong parent task id".into());
        return false;
    }
    let required = [
        "issue",
        "plan step",
        "branch",
        "worktree",
        "head",
        "clean/index",
        "evidence",
        "next action",
    ];
    if field(line, "operation") != Some(operation)
        || field(line, "delivery") != Some("confirmed")
        || field(line, "task surface") != Some("codex task/thread")
        || required.iter().any(|name| invalid_value(field(line, name)))
    {
        errors.push("goal pre-delivery report is missing required pre-delivery fields".into());
        return false;
    }
    matches_key(line, key, errors)
}
fn post_result_is_confirmed(
    line: &str,
    operation: &str,
    source: &str,
    key: Option<&str>,
    errors: &mut Vec<String>,
) -> bool {
    if field(line, "parent task") != Some(source) {
        errors.push("goal report names the wrong parent task id".into());
        return false;
    }
    if !matches_key(line, key, errors) {
        return false;
    }
    if field(line, "operation") != Some(operation)
        || field(line, "delivery") != Some("confirmed")
        || field(line, "task surface") != Some("codex task/thread")
        || invalid_value(field(line, "exact tool result"))
    {
        errors.push("goal post-result report is prose-only or missing an exact tool result".into());
        return false;
    }
    true
}
fn is_local_agent_route(line: &str) -> bool {
    line.strip_prefix("parent route: ")
        .and_then(|route| route.split([';', ',', ' ']).next())
        .is_some_and(is_local_task_target)
        || line.match_indices("agents.send_message").any(|(index, _)| {
            let prefix = &line[..index];
            !prefix
                .rsplit_once(". ")
                .map_or(prefix, |(_, sentence)| sentence)
                .rsplit([';', ':'])
                .next()
                .is_some_and(|clause| clause.contains("must not use"))
        })
}
fn matches_key(line: &str, key: Option<&str>, errors: &mut Vec<String>) -> bool {
    let matches = key.is_some_and(|value| field(line, "transition key") == Some(value));
    if !matches {
        errors.push("goal receipt does not match its stable transition key".into());
    }
    matches
}
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    line.split(';').map(str::trim).find_map(|part| {
        part.strip_prefix(&prefix).or_else(|| {
            part.split_once(": ")
                .filter(|(label, _)| {
                    matches!(
                        *label,
                        "goal control state"
                            | "parent goal pre-delivery"
                            | "parent goal post-result"
                    )
                })
                .and_then(|(_, value)| value.strip_prefix(&prefix))
        })
    })
}
fn invalid_value(value: Option<&str>) -> bool {
    value.is_none_or(|item| {
        matches!(item, "" | "false" | "unavailable" | "none") || item.contains(" unavailable")
    })
}
fn is_goal_reporting(line: &str) -> bool {
    "source thread id:|goal tool call:|parent goal pre-delivery:|parent goal post-result:"
        .split('|')
        .any(|prefix| line.starts_with(prefix))
}
fn is_lane_boundary(lines: &[&str], tables: &[ClassificationTable], index: usize) -> bool {
    owner_at(tables, index).is_some()
        || is_legacy_ownership_boundary(lines[index])
        || table_ownership_boundary(tables, lines, index)
        || (field_value(lines[index], "pr").is_some()
            && tables.iter().any(|table| table.start < index)
            && !child_table_owns_handoff_pr(tables, lines, index))
}
