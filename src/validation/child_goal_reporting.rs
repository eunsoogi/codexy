use std::collections::BTreeSet;

pub(super) fn check(evidence: &str) -> Vec<String> {
    let text = evidence.to_ascii_lowercase();
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    let mut errors = Vec::new();
    let mut start = 0;
    for end in 1..=lines.len() {
        if end == lines.len() || is_lane_boundary(lines[end]) {
            if is_child_owned(lines[start]) {
                errors.extend(check_lane(&lines[start..end]));
            }
            start = end;
        }
    }
    errors
}

fn check_lane(lines: &[&str]) -> Vec<String> {
    if !lines.iter().any(|line| {
        line.starts_with("source thread id:")
            || line.starts_with("goal tool call:")
            || line.starts_with("parent goal pre-delivery:")
            || line.starts_with("parent goal post-result:")
    }) {
        return Vec::new();
    }
    let Some(source) = lines
        .iter()
        .find_map(|line| line.strip_prefix("source thread id: "))
        .filter(|value| !value.is_empty())
    else {
        return vec!["child goal reporting requires source_thread_id delegation evidence".into()];
    };
    if is_local_agent_target(source) {
        return vec!["source_thread_id must name a Codex task id, not a local agent target".into()];
    }
    let control = format!("goal control state: source_thread_id={source}");
    let mut errors = Vec::new();
    let mut key = None;
    let mut pending = None;
    let mut confirmed_pre = None;
    let mut seen_calls = BTreeSet::new();

    for line in lines {
        if is_local_agent_route(line) {
            errors.push("child goal reporting must not use local agents /root routing".into());
        }
        if let Some(value) = line.strip_prefix("goal transition key: ") {
            key = valid_transition_key(value).then_some(value);
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
            let valid_key = key.is_some_and(|value| key_matches(value, operation));
            if !lines.contains(&control.as_str()) || !valid_key {
                errors.push("goal operation lacks a stable transition key and exact source_thread_id control state".into());
            }
            if needs_pre_delivery(operation) {
                match confirmed_pre {
                    Some((pre_operation, pre_key))
                        if pre_operation == operation && pre_key == key => {}
                    Some(_) => errors.push(
                        "pre-delivery receipt does not match the goal call stable transition key"
                            .into(),
                    ),
                    None => errors.push(pre_delivery_error(operation)),
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
        .map(|value| value.split(';').next().unwrap_or(value))
}

fn valid_transition_key(key: &str) -> bool {
    key.split(':').count() == 3 && key.split(':').all(|part| !part.is_empty())
}

fn key_matches(key: &str, operation: &str) -> bool {
    let action = match operation {
        "update_goal(blocked)" => "blocked",
        "update_goal(complete)" => "complete",
        operation => operation,
    };
    key.contains(action)
}

fn needs_pre_delivery(operation: &str) -> bool {
    matches!(
        operation,
        "create_goal" | "update_goal(complete)" | "update_goal(blocked)"
    )
}

fn pre_delivery_error(operation: &str) -> String {
    match operation {
        "update_goal(blocked)" => {
            "blocked goal operation precedes confirmed parent delivery".into()
        }
        "update_goal(complete)" => {
            "complete goal operation precedes confirmed parent delivery".into()
        }
        _ => "goal operation requires confirmed pre-delivery parent report".into(),
    }
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
    line.contains("agents.send_message") && line.contains("/root")
}

fn matches_key(line: &str, key: Option<&str>, errors: &mut Vec<String>) -> bool {
    if key.is_some_and(|value| field(line, "transition key") == Some(value)) {
        true
    } else {
        errors.push("goal receipt does not match its stable transition key".into());
        false
    }
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    line.split(';').map(str::trim).find_map(|part| {
        part.strip_prefix(&prefix).or_else(|| {
            part.split_once(": ")
                .filter(|(label, _)| label.starts_with("parent goal "))
                .and_then(|(_, value)| value.strip_prefix(&prefix))
        })
    })
}

fn invalid_value(value: Option<&str>) -> bool {
    value.is_none_or(|item| {
        item.is_empty()
            || matches!(item, "false" | "unavailable" | "none")
            || item.contains(" unavailable")
    })
}

fn is_local_agent_target(value: &str) -> bool {
    value == "/root" || value.starts_with("agents.") || value.contains("send_message")
}

fn is_lane_boundary(line: &str) -> bool {
    line.contains("lane ownership:") || line.contains("owner decision:")
}

fn is_child_owned(line: &str) -> bool {
    line.contains("lane ownership: child-owned") || line.contains("owner decision: child-owned")
}
