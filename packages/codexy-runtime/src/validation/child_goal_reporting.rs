mod negation;
mod objective;

pub(super) fn check(evidence: &str) -> Vec<String> {
    let active = super::child_lifecycle_events::active_lines(evidence);
    let lines = active
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    if is_clear_child_implementation(&lines)
        && lines
            .iter()
            .any(|line| negation::prohibited_goal_tools(line))
    {
        return vec![
            "clear delegated implementation must not prohibit available goal tools".into(),
        ];
    }
    if let Some(error) = goal_sequence_error(&lines) {
        return vec![error.into()];
    }
    Vec::new()
}

#[derive(Clone)]
enum GoalObservation {
    Active(Option<String>),
    AllowsCreate,
    Invalid,
}

fn goal_sequence_error(lines: &[&str]) -> Option<&'static str> {
    let mut observation = None;
    let mut saw_get = false;
    let mut created = None;
    let mut active_readback = false;
    let authorized = objective::authorized(lines);
    let mut transition_keys = std::collections::HashSet::new();
    for line in lines {
        let line = super::child_terminal_handoff::without_metadata_prefix(line).trim();
        if let Some(current) = get_goal_observation(line) {
            saw_get = true;
            let Some(key) = field(line, "transition key") else {
                return Some("get_goal receipt requires a unique transition key");
            };
            if !transition_keys.insert(key) {
                return Some("get_goal receipt transition key must not be repeated");
            }
            if let Some(expected) = created.as_deref() {
                let GoalObservation::Active(Some(actual)) = &current else {
                    return Some("create_goal requires an active readback after create_goal");
                };
                if actual != expected {
                    return Some("active goal readback objective must match create_goal");
                }
                active_readback = true;
            }
            observation = Some(current);
        } else if is_create_pre_delivery(line) {
            if !matches!(observation, Some(GoalObservation::AllowsCreate)) {
                return Some(create_without_authority_error(&observation));
            }
        } else if let Some(objective) = create_goal_objective(line) {
            if created.is_some() {
                return Some("create_goal must not be called more than once for one assignment");
            }
            if !matches!(observation, Some(GoalObservation::AllowsCreate)) {
                return Some(create_without_authority_error(&observation));
            }
            if authorized.is_none_or(|authorized| objective != authorized) {
                return Some(
                    "create_goal objective must exactly match the authorized assignment objective",
                );
            }
            created = Some(objective.to_owned());
            observation = None;
        }
    }
    if !saw_get {
        return None;
    }
    if created.is_none() && matches!(observation, Some(GoalObservation::AllowsCreate)) {
        return Some("clear delegated assignment requires an actual create_goal tool call");
    }
    if created.is_some() && !active_readback {
        return Some("create_goal requires an active readback after create_goal");
    }
    None
}

fn create_without_authority_error(observation: &Option<GoalObservation>) -> &'static str {
    if matches!(observation, Some(GoalObservation::Active(_))) {
        "active goal must be preserved and must not be replaced by create_goal"
    } else {
        "create_goal requires a valid authoritative get_goal result"
    }
}

fn get_goal_observation(line: &str) -> Option<GoalObservation> {
    if !line.starts_with("parent goal post-result:") || field(line, "operation") != Some("get_goal")
    {
        return None;
    }
    if !valid_receipt(line) {
        return Some(GoalObservation::Invalid);
    }
    let result = field(line, "exact tool result")?;
    let Ok(value) = serde_json::from_str::<serde_json::Value>(result) else {
        return Some(match result {
            "null" | "complete" | "status=complete" => GoalObservation::AllowsCreate,
            "active" | "status=active" => GoalObservation::Active(None),
            _ => GoalObservation::Invalid,
        });
    };
    if value.is_null() {
        return Some(GoalObservation::AllowsCreate);
    }
    if value.get("goal").is_some_and(serde_json::Value::is_null) {
        return Some(
            match value.get("status").and_then(serde_json::Value::as_str) {
                None | Some("complete") => GoalObservation::AllowsCreate,
                _ => GoalObservation::Invalid,
            },
        );
    }
    let goal = value.get("goal").unwrap_or(&value);
    let status = value
        .get("goal")
        .and_then(|goal| goal.get("status"))
        .or_else(|| value.get("status"))
        .and_then(serde_json::Value::as_str);
    Some(match status {
        Some("active") => GoalObservation::Active(
            goal.get("objective")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        ),
        Some("complete") => GoalObservation::AllowsCreate,
        _ => GoalObservation::Invalid,
    })
}

fn valid_receipt(line: &str) -> bool {
    field(line, "parent task").is_some_and(|value| !value.is_empty())
        && field(line, "delivery") == Some("confirmed")
        && field(line, "task surface") == Some("codex task/thread")
        && field(line, "transition key").is_some_and(|value| !value.is_empty())
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    let payload = line.split_once(": ").map_or(line, |(_, payload)| payload);
    if name == "exact tool result" {
        return payload
            .split_once(&prefix)
            .and_then(|(_, value)| value.rsplit_once("; parent task="))
            .map(|(value, _)| value.trim());
    }
    payload
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&prefix))
}

fn is_create_pre_delivery(line: &str) -> bool {
    line.starts_with("parent goal pre-delivery:") && field(line, "operation") == Some("create_goal")
}

fn create_goal_objective(line: &str) -> Option<&str> {
    line.strip_prefix("goal tool call: create_goal")?
        .split_once("objective=")
        .map(|(_, objective)| objective.trim().trim_end_matches(')'))
        .filter(|objective| !objective.is_empty())
}

fn is_clear_child_implementation(lines: &[&str]) -> bool {
    lines
        .iter()
        .any(|line| line.trim() == "lane ownership: child-owned")
        && classification_value(lines, "lane type").is_some_and(|value| {
            value
                .split_whitespace()
                .any(|word| word == "implementation")
        })
        && classification_value(lines, "atomic scope").is_some_and(|value| {
            value
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains("issue-sized")
        })
}

fn classification_value<'a>(lines: &'a [&str], key: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| {
        let line = line.trim();
        line.strip_prefix('|')
            .and_then(|line| line.strip_suffix('|'))
            .and_then(|line| {
                let mut fields = line.split('|').map(str::trim);
                (fields.next() == Some(key))
                    .then(|| fields.next())
                    .flatten()
            })
            .or_else(|| line.strip_prefix(&format!("{key}: ")))
    })
}
