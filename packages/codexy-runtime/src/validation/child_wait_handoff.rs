mod negation;
pub(super) mod wait_taxonomy;

use super::{child_lifecycle_events, child_terminal_handoff::field};
use wait_taxonomy::{WaitDisposition, classify_producer};

/// Validates the wait receipt independently of native goal state or transitions.
pub(super) fn check(_plugin_root: &std::path::Path, evidence: &str) -> Vec<String> {
    let events = child_lifecycle_events::active_lines(evidence);
    events
        .iter()
        .filter(|event| event.text.starts_with("nonterminal wait handoff:"))
        .filter_map(|event| {
            let line = &event.text;
            (invalid_field(field(line, "state fingerprint"))
                || !field(line, "producer state")
                    .and_then(classify_producer)
                    .is_some_and(|state| state == WaitDisposition::Nonterminal)
                || invalid_field(field(line, "wake route"))
                || field(line, "ownership") != Some("retained")
                || field(line, "plan state") != Some("active")
                || field(line, "return control") != Some("confirmed"))
            .then_some("nonterminal wait handoff requires a stable fingerprint, nonterminal producer, available wake route, retained ownership, active plan state, and confirmed return control".into())
        })
        .collect()
}

fn invalid_field(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.is_empty() || matches!(value, "none" | "unavailable"))
}
