mod fields;
pub(super) use fields::field;
use fields::{confirmed_handoff, valid_value};

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
    let errors: Vec<String> = lines
        .iter()
        .filter_map(|line| handoffs.observe(without_metadata_prefix(line), source))
        .map(str::to_owned)
        .collect();
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
                || matches!(self.state, HandoffState::Ready | HandoffState::Stopped)
            {
                self.state = HandoffState::Missing;
                return Some(
                    "terminal parent handoff must not be repeated before terminal transition",
                );
            }
            self.phase = phase;
            // Receipt validity is independent of any native goal operation; it does not prove the work.
            self.completed = phase;
            self.state = if phase {
                HandoffState::Completed
            } else {
                HandoffState::Ready
            };
            return None;
        }
        if line.starts_with("idle child state:") {
            return (!(self.completed && field(line, "plan state") == Some("idle")))
                .then_some("idle child state requires a valid phase handoff");
        }
        if line.starts_with("child tool call: wait_threads")
            && source.is_some_and(|parent| field(line, "target") == Some(parent))
        {
            return Some("phase result submission must not wait_threads on its parent");
        }
        if line.starts_with("terminal child transition:")
            && field(line, "action").is_none_or(str::is_empty)
        {
            return Some("terminal child transition requires one unambiguous action");
        }
        if let Some(transition) = terminal_transition(line) {
            let valid = matches!(self.state, HandoffState::Ready)
                || matches!(transition, "ownership release")
                    && matches!(self.state, HandoffState::Stopped)
                || transition == "stop" && matches!(self.state, HandoffState::Completed);
            if valid {
                self.state = if matches!(transition, "stop") && !self.phase {
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
        && field(line, "return control") == Some("confirmed")
}

fn terminal_transition(line: &str) -> Option<&str> {
    line.starts_with("terminal child transition:")
        .then(|| field(line, "action"))
        .flatten()
        .filter(|action| ["stop", "archive", "ownership release", "blocked"].contains(action))
}
