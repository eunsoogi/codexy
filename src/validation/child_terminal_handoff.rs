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

#[derive(Default)]
pub(super) struct TerminalHandoffs(usize);

impl TerminalHandoffs {
    pub(super) fn observe(&mut self, line: &str, source: &str) -> Option<&'static str> {
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
    matches!(
        line.strip_prefix("goal tool call: "),
        Some("update_goal(complete)" | "update_goal(blocked)")
    ) || line
        .strip_prefix("terminal child transition: action=")
        .is_some_and(|action| matches!(action, "stop" | "archive" | "ownership release"))
}

fn confirmed_handoff(line: &str, source: &str) -> bool {
    line.strip_prefix("terminal parent handoff:")
        .is_some_and(|_| {
            field(line, "parent task") == Some(source)
                && field(line, "delivery") == Some("confirmed")
                && field(line, "task surface") == Some("codex task/thread")
                && REQUIRED_FIELDS
                    .iter()
                    .all(|name| field(line, name).is_some_and(valid_value))
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

fn valid_value(value: &str) -> bool {
    !value.is_empty()
        && !matches!(value, "false" | "unavailable" | "none")
        && !value.contains(" unavailable")
}
