use super::{GoalState, Operation};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RecordKind {
    Pre,
    Post,
}

pub(super) struct Call<'a> {
    pub(super) index: usize,
    pub(super) operation: Operation,
    pub(super) key: String,
    pub(super) objective: Option<&'a str>,
}

pub(super) fn calls<'a>(
    lines: &[&str],
    raw_lines: &'a [&'a str],
) -> Result<Vec<Call<'a>>, &'static str> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            raw_lines
                .get(index)
                .and_then(|raw_line| parse_call(index, normalized(line), raw_line))
        })
        .collect()
}

fn parse_call<'a>(
    index: usize,
    line: &str,
    raw_line: &'a str,
) -> Option<Result<Call<'a>, &'static str>> {
    let payload = line.strip_prefix("goal tool call: ")?;
    let operation = payload
        .rsplit_once("; parent task=")
        .map_or(payload, |(operation, _)| operation)
        .trim();
    let raw_payload = strip_prefix_ascii_case(raw_line, "goal tool call: ")?;
    let raw_operation = rsplit_once_ascii_case(raw_payload, "; parent task=")
        .map_or(raw_payload, |(operation, _)| operation)
        .trim();
    let (operation, objective) = if operation == "get_goal" {
        (Operation::Get, None)
    } else if let Some(value) = operation.strip_prefix("create_goal(objective=") {
        let Some(_) = value.strip_suffix(')') else {
            return Some(Err("goal tool call has malformed operation or objective"));
        };
        let Some(raw_value) = raw_operation
            .get("create_goal(objective=".len()..)
            .and_then(|value| value.strip_suffix(')'))
        else {
            return Some(Err("goal tool call has malformed operation or objective"));
        };
        (Operation::Create, Some(raw_value.trim()))
    } else {
        return Some(Err("goal tool call has malformed operation or objective"));
    };
    Some(
        field(line, "transition key")
            .filter(|value| !value.is_empty())
            .map(|key| Call {
                index,
                operation,
                key: key.to_owned(),
                objective,
            })
            .ok_or("goal tool call requires a transition key"),
    )
}

pub(super) fn receipt_operation(line: &str) -> Option<(RecordKind, Operation)> {
    let kind = if line.starts_with("parent goal pre-delivery:") {
        RecordKind::Pre
    } else if line.starts_with("parent goal post-result:") {
        RecordKind::Post
    } else {
        return None;
    };
    parsed_operation(line).map(|operation| (kind, operation))
}

fn parsed_operation(line: &str) -> Option<Operation> {
    match field(line, "operation")? {
        "get_goal" => Some(Operation::Get),
        "create_goal" => Some(Operation::Create),
        _ => None,
    }
}

pub(super) fn require_parent(line: &str, source: &str) -> Result<(), &'static str> {
    (field(normalized(line), "parent task") == Some(source))
        .then_some(())
        .ok_or("goal transition parent task must match source thread id")
}

pub(super) fn require_confirmed(line: &str) -> Result<(), &'static str> {
    let line = normalized(line);
    (field(line, "delivery") == Some("confirmed")
        && field(line, "task surface") == Some("codex task/thread"))
    .then_some(())
    .ok_or("goal receipt requires confirmed Codex task delivery")
}

pub(super) fn require_create_pre_fields(line: &str, authorized: &str) -> Result<(), &'static str> {
    let required = [
        "issue/pr",
        "parent task",
        "plan step",
        "branch",
        "worktree",
        "head",
        "clean/index",
        "evidence",
        "next action",
        "transition key",
    ];
    (required
        .iter()
        .all(|name| field(line, name).is_some_and(valid_value))
        && field(line, "pending objective") == Some(authorized))
    .then_some(())
    .ok_or("create_goal pre-delivery receipt is missing required bound fields")
}

pub(super) fn goal_state(result: &str) -> GoalState {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(result) else {
        return match result {
            "null" | "complete" | "status=complete" => GoalState::AllowsCreate,
            "active" | "status=active" => GoalState::Active(None),
            _ => GoalState::Invalid,
        };
    };
    if value.is_null() || value.get("goal").is_some_and(serde_json::Value::is_null) {
        return match value.get("status") {
            None => GoalState::AllowsCreate,
            Some(status)
                if status
                    .as_str()
                    .is_some_and(|status| status.eq_ignore_ascii_case("complete")) =>
            {
                GoalState::AllowsCreate
            }
            _ => GoalState::Invalid,
        };
    }
    let goal = value.get("goal").unwrap_or(&value);
    let status = goal
        .get("status")
        .or_else(|| value.get("status"))
        .and_then(serde_json::Value::as_str);
    match status {
        Some(status) if status.eq_ignore_ascii_case("active") => GoalState::Active(
            goal.get("objective")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        ),
        Some(status) if status.eq_ignore_ascii_case("complete") => GoalState::AllowsCreate,
        _ => GoalState::Invalid,
    }
}

pub(super) fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    let payload = line.split_once(": ").map_or(line, |(_, payload)| payload);
    if matches!(name, "exact tool result" | "pending objective") {
        return split_once_ascii_case(payload, &prefix)
            .and_then(|(_, value)| rsplit_once_ascii_case(value, "; parent task="))
            .map(|(value, _)| value.trim());
    }
    let mut values = payload
        .split(';')
        .map(str::trim)
        .filter_map(|part| strip_prefix_ascii_case(part, &prefix));
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn valid_value(value: &str) -> bool {
    !value.is_empty() && !matches!(value, "false" | "unavailable" | "none")
}

pub(super) fn normalized(line: &str) -> &str {
    super::super::super::child_terminal_handoff::without_metadata_prefix(line).trim()
}

fn strip_prefix_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &value[prefix.len()..])
}

fn split_once_ascii_case<'a>(value: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    let index = value
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())?;
    Some((&value[..index], &value[index + needle.len()..]))
}

fn rsplit_once_ascii_case<'a>(value: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    let index = value
        .to_ascii_lowercase()
        .rfind(&needle.to_ascii_lowercase())?;
    Some((&value[..index], &value[index + needle.len()..]))
}
