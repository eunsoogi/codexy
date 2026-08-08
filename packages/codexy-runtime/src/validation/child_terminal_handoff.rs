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
        .filter_map(|line| {
            handoffs
                .observe(without_metadata_prefix(line), source)
                .map(str::to_owned)
        })
        .collect()
}

pub(super) fn without_metadata_prefix(line: &str) -> &str {
    let line = line.trim_start();
    let rest = line.trim_start_matches(|character: char| character.is_ascii_digit());
    rest.strip_prefix(". ")
        .or_else(|| rest.strip_prefix(") "))
        .or_else(|| line.strip_prefix("- "))
        .or_else(|| line.strip_prefix("* "))
        .unwrap_or(line)
}

pub(super) fn is_local_task_target(value: &str) -> bool {
    value == "/root" || value.starts_with("agents.") || value.contains("send_message")
}

#[derive(Default)]
enum TerminalHandoffs {
    #[default]
    Missing,
    Ready,
    Duplicate,
    Stopped,
}

impl TerminalHandoffs {
    fn observe(&mut self, line: &str, source: Option<&str>) -> Option<&'static str> {
        if line.starts_with("terminal parent handoff:") {
            if confirmed_handoff(line, source) {
                if matches!(self, Self::Ready | Self::Duplicate | Self::Stopped) {
                    *self = Self::Duplicate;
                    return Some(
                        "terminal parent handoff must not be repeated before terminal transition",
                    );
                }
                *self = Self::Ready;
                return None;
            }
            return Some("terminal parent handoff is missing required confirmed delivery fields");
        }
        if let Some(transition) = terminal_transition(line) {
            let valid = match transition {
                TerminalTransition::Goal
                | TerminalTransition::Archive
                | TerminalTransition::Blocked => {
                    matches!(self, Self::Ready)
                }
                TerminalTransition::Stop => matches!(self, Self::Ready),
                TerminalTransition::OwnershipRelease => matches!(self, Self::Ready | Self::Stopped),
            };
            if valid {
                *self = match transition {
                    TerminalTransition::Stop => Self::Stopped,
                    _ => Self::Missing,
                };
            }
            return (!valid).then_some(
                "terminal child transition requires exactly one confirmed terminal parent handoff",
            );
        }
        None
    }
}

#[derive(Clone, Copy)]
enum TerminalTransition {
    Goal,
    Stop,
    Archive,
    OwnershipRelease,
    Blocked,
}

fn terminal_transition(line: &str) -> Option<TerminalTransition> {
    line.strip_prefix("goal tool call: ")
        .and_then(|value| value.split(';').next())
        .filter(|operation| is_terminal_goal_call(operation))
        .map(|_| TerminalTransition::Goal)
        .or_else(|| {
            line.strip_prefix("terminal child transition: action=")
                .and_then(|value| value.split(';').next())
                .and_then(|action| match action {
                    "stop" => Some(TerminalTransition::Stop),
                    "archive" => Some(TerminalTransition::Archive),
                    "ownership release" => Some(TerminalTransition::OwnershipRelease),
                    "blocked" => Some(TerminalTransition::Blocked),
                    _ => None,
                })
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
            let child_task = field(line, "child task");
            !parent_task.is_some_and(is_local_task_target)
                && !parent_task.is_some_and(is_placeholder_task)
                && !child_task.is_some_and(is_local_task_target)
                && !child_task.is_some_and(is_placeholder_task)
                && source.is_none_or(|expected| parent_task == Some(expected))
                && field(line, "delivery") == Some("confirmed")
                && field(line, "task surface") == Some("codex task/thread")
                && REQUIRED_FIELDS
                    .iter()
                    .all(|name| field(line, name).is_some_and(valid_value))
        })
}

fn is_placeholder_task(value: &str) -> bool {
    matches!(
        value,
        "codex task/thread" | "parent task" | "child task" | "task/thread"
    )
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
