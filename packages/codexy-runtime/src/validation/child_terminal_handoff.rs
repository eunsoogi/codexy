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

/// Checks terminal receipts and honest finite-phase submission without parent-response waits.
pub(super) fn check(lines: &[&str], source: Option<&str>) -> Vec<String> {
    let mut handoffs = TerminalHandoffs::default();
    let mut errors: Vec<String> = lines
        .iter()
        .filter_map(|line| {
            handoffs
                .observe(without_metadata_prefix(line), source)
                .map(str::to_owned)
        })
        .collect();
    if matches!(handoffs.state, HandoffState::AwaitingResult) {
        errors.push("phase completion requires its native goal result".into());
    }
    errors
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
struct TerminalHandoffs {
    state: HandoffState,
    phase: bool,
    completed: bool,
    events: std::collections::BTreeSet<String>,
}

#[derive(Default)]
enum HandoffState {
    #[default]
    Missing,
    Ready,
    AwaitingResult,
    Completed,
    Stopped,
}

impl TerminalHandoffs {
    fn observe(&mut self, line: &str, source: Option<&str>) -> Option<&'static str> {
        if line.starts_with("idle wait handoff:") || line.starts_with("terminal parent handoff:") {
            let phase = line.starts_with("idle wait handoff:");
            if !confirmed_handoff(line, source) || (phase && !valid_phase(line)) {
                self.state = HandoffState::Missing;
                return Some(
                    "terminal parent handoff is missing required confirmed delivery fields or satisfied phase criteria",
                );
            }
            // Event identity survives consumption, so replay cannot authorize another goal call.
            if !self
                .events
                .insert(field(line, "event id").unwrap_or_default().to_owned())
                || matches!(
                    self.state,
                    HandoffState::Ready | HandoffState::Stopped | HandoffState::AwaitingResult
                )
            {
                self.state = HandoffState::Missing;
                return Some(
                    "terminal parent handoff must not be repeated before terminal transition",
                );
            }
            self.phase = phase;
            self.completed = false;
            self.state = HandoffState::Ready;
            return None;
        }
        if line.starts_with("goal tool result:") && self.phase {
            // Tool acceptance and successful completion differ; phase stops retain exit ownership.
            if matches!(self.state, HandoffState::AwaitingResult)
                && field(line, "operation").is_some_and(is_complete_goal_call)
            {
                let status = field(line, "status")?;
                self.completed = status == "complete";
                self.state = if self.completed {
                    HandoffState::Completed
                } else {
                    HandoffState::Missing
                };
                return None;
            }
            return Some("phase goal result requires its matching completion call");
        }
        if line.starts_with("idle child state:") {
            return (!(self.completed
                && field(line, "goal state") == Some("complete")
                && field(line, "plan state") == Some("idle")))
            .then_some("idle child state requires a successful phase goal result");
        }
        if line.starts_with("child tool call: wait_threads")
            && source.is_some_and(|parent| field(line, "target") == Some(parent))
        {
            return Some("phase result submission must not wait_threads on its parent");
        }
        if let Some(transition) = terminal_transition(line) {
            let valid = (matches!(self.state, HandoffState::Ready)
                && (!self.phase || transition == "goal"))
                || matches!(transition, "ownership release")
                    && matches!(self.state, HandoffState::Stopped)
                || transition == "stop" && matches!(self.state, HandoffState::Completed);
            if valid {
                self.state = if matches!(transition, "goal") && self.phase {
                    if !line
                        .strip_prefix("goal tool call: ")
                        .and_then(|value| value.split(';').next())
                        .is_some_and(is_complete_goal_call)
                    {
                        return Some("idle phase handoff authorizes only goal completion");
                    }
                    HandoffState::AwaitingResult
                } else if matches!(transition, "stop") && !self.phase {
                    HandoffState::Stopped
                } else {
                    HandoffState::Missing
                };
            }
            return (!valid).then_some(
                "terminal child transition requires exactly one confirmed terminal parent handoff",
            );
        }
        None
    }
}

fn is_complete_goal_call(operation: &str) -> bool {
    matches!(
        operation,
        "update_goal(complete)" | "update_goal(status=\"complete\")"
    )
}

fn valid_phase(line: &str) -> bool {
    [
        "phase objective",
        "state fingerprint",
        "producer state",
        "wake route",
    ]
    .iter()
    .all(|name| field(line, name).is_some_and(valid_value))
        && field(line, "phase criteria") == Some("satisfied")
        && field(line, "ownership") == Some("retained")
        && field(line, "issue state") == Some("not complete")
        && field(line, "goal transition") == Some("complete")
        && field(line, "return control") == Some("confirmed")
}

fn terminal_transition(line: &str) -> Option<&str> {
    if line
        .strip_prefix("goal tool call: ")
        .and_then(|value| value.split(';').next())
        .is_some_and(is_terminal_goal_call)
    {
        return Some("goal");
    }
    line.strip_prefix("terminal child transition: action=")
        .and_then(|value| value.split(';').next())
        .filter(|action| {
            matches!(
                *action,
                "stop" | "archive" | "ownership release" | "blocked"
            )
        })
}

pub(super) fn is_terminal_goal_call(operation: &str) -> bool {
    is_complete_goal_call(operation) || is_blocked_goal_call(operation)
}

pub(super) fn is_blocked_goal_call(operation: &str) -> bool {
    matches!(
        operation,
        "update_goal(blocked)" | "update_goal(status=\"blocked\")"
    )
}

fn confirmed_handoff(line: &str, source: Option<&str>) -> bool {
    // A receipt must identify both actual tasks; placeholders are not transport evidence.
    ["parent task", "child task"].iter().all(|name| {
        field(line, name)
            .is_some_and(|task| !is_local_task_target(task) && !is_placeholder_task(task))
    }) && source.is_none_or(|expected| field(line, "parent task") == Some(expected))
        && delivery_supported(line)
        && REQUIRED_FIELDS
            .iter()
            .all(|name| field(line, name).is_some_and(valid_value))
}

// Host final-result submission is an honest fallback, never a confirmed message receipt.
fn delivery_supported(line: &str) -> bool {
    (field(line, "delivery") == Some("confirmed")
        && field(line, "task surface") == Some("codex task/thread")
        && field(line, "message permission") != Some("absent"))
        || (line.starts_with("idle wait handoff:")
            && field(line, "delivery") == Some("unconfirmed")
            && field(line, "task surface") == Some("host terminal result")
            && field(line, "message permission") == Some("absent"))
}

fn is_placeholder_task(value: &str) -> bool {
    matches!(
        value,
        "codex task/thread" | "parent task" | "child task" | "task/thread"
    )
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    let mut values = line
        .split_once(": ")
        .map_or(line, |(_, value)| value)
        .split(';')
        .map(str::trim)
        .filter_map(|part| part.strip_prefix(&prefix));
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn valid_value(value: &str) -> bool {
    !value.is_empty()
        && !matches!(value, "false" | "unavailable" | "none")
        && !value.contains(" unavailable")
}
