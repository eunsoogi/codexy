use super::*;

const NONTERMINAL_PRODUCERS: &[&str] = &[
    "sentinel-running",
    "child-pending",
    "ci-queued",
    "connector-review-pending",
    "reviewer-pending",
    "parent-authorization-pending",
    "dependency-integration-pending",
    "resource-slot-pending",
    "alternate-evidence-pending",
    "async-tool-pending",
    "event-idle-child",
];

pub(in crate::validation) fn classify_producer(value: &str) -> Option<WaitDisposition> {
    if NONTERMINAL_PRODUCERS.contains(&value) {
        return Some(WaitDisposition::Nonterminal);
    }
    // A natural description must name a live producer and must not negate it or describe actionable feedback.
    let tokens = words(value);
    (!tokens.iter().any(|word| is_negation(word))
        && classify_wait_text(value) == Some(WaitDisposition::Nonterminal)
        && event_words(value)
            .iter()
            .any(|event| is_nonterminal_wait(event)))
    .then_some(WaitDisposition::Nonterminal)
}
