const REQUIRED_FIELDS: &[&str] = &[
    "event id",
    "issue/pr",
    "child task",
    "parent task",
    "branch",
    "worktree",
    "head",
    "clean/index",
    "last proof",
    "current gate",
    "preserved reservation/artifacts",
    "parent next action",
];

pub(super) fn check(lines: &[&str], source: Option<&str>) -> Vec<String> {
    let mut handoffs = TerminalHandoffs::default();
    lines
        .iter()
        .filter_map(|line| handoffs.observe(line, source).map(str::to_owned))
        .collect()
}

pub(super) fn is_local_task_target(value: &str) -> bool {
    value == "/root" || value.starts_with("agents.") || value.contains("send_message")
}

#[derive(Default)]
struct TerminalHandoffs(usize);

impl TerminalHandoffs {
    fn observe(&mut self, line: &str, source: Option<&str>) -> Option<&'static str> {
        if line.starts_with("terminal parent handoff:") {
            if confirmed_handoff(line, source) {
                self.0 += 1;
                return None;
            }
            return Some("terminal parent handoff is missing required confirmed delivery fields");
        }
        if is_terminal_transition(line) {
            let valid = self.0 == 1;
            self.0 = 0;
            return (!valid).then_some(
                "terminal child transition requires exactly one confirmed terminal parent handoff",
            );
        }
        None
    }
}

fn is_terminal_transition(line: &str) -> bool {
    line.strip_prefix("goal tool call: ")
        .and_then(|value| value.split(';').next())
        .is_some_and(is_terminal_goal_call)
        || line
            .strip_prefix("terminal child transition: action=")
            .and_then(|value| value.split(';').next())
            .is_some_and(|action| {
                matches!(action, "stop" | "archive" | "ownership release" | "blocked")
            })
}

pub(super) fn is_terminal_goal_call(operation: &str) -> bool {
    matches!(
        operation,
        "update_goal(complete)"
            | "update_goal(blocked)"
            | "update_goal(status=\"complete\")"
            | "update_goal(status=\"blocked\")"
    )
}

fn confirmed_handoff(line: &str, source: Option<&str>) -> bool {
    line.strip_prefix("terminal parent handoff:")
        .is_some_and(|_| {
            let parent_task = field(line, "parent task");
            !parent_task.is_some_and(is_local_task_target)
                && !parent_task.is_some_and(is_placeholder_task)
                && source.is_none_or(|expected| parent_task == Some(expected))
                && field(line, "delivery") == Some("confirmed")
                && field(line, "task surface") == Some("codex task/thread")
                && REQUIRED_FIELDS
                    .iter()
                    .all(|name| field(line, name).is_some_and(valid_value))
        })
}

fn is_placeholder_task(value: &str) -> bool {
    matches!(value, "codex task/thread" | "parent task" | "task/thread")
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    line.split_once(": ")
        .map_or(line, |(_, value)| value)
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&prefix))
}

fn valid_value(value: &str) -> bool {
    !value.is_empty()
        && !matches!(value, "false" | "unavailable" | "none")
        && !value.contains(" unavailable")
}
