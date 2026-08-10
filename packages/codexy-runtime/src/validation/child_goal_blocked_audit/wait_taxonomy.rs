#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::validation) enum WaitDisposition {
    Nonterminal,
    Actionable,
}

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

const REVIEW_SUBJECTS: &[&str] = &[
    "review",
    "reviewer",
    "review feedback",
    "review comment",
    "requested changes",
    "changes requested",
];

const ACTIONABLE_REVIEW_STATES: &[&str] = &[
    "actionable feedback",
    "actionable review",
    "changes requested",
    "requested changes",
    "suggestion",
    "unresolved",
    "resolution required",
];

const NON_ACTIONABLE_REVIEW_STATES: &[&str] = &[
    "no actionable feedback",
    "no actionable review feedback",
    "no feedback",
    "no review feedback",
];

const PENDING_STATES: &[&str] = &[
    "pending",
    "waiting",
    "awaiting",
    "in progress",
    "processing",
    "not returned",
    "not yet returned",
    "has not returned",
    "hasn't returned",
];

pub(in crate::validation) fn classify_producer(value: &str) -> Option<WaitDisposition> {
    NONTERMINAL_PRODUCERS
        .contains(&value)
        .then_some(WaitDisposition::Nonterminal)
}

pub(in crate::validation) fn classify_reviewer_text(text: &str) -> Option<WaitDisposition> {
    if !contains_any(text, REVIEW_SUBJECTS) {
        return None;
    }
    if contains_any(text, ACTIONABLE_REVIEW_STATES)
        && !contains_any(text, NON_ACTIONABLE_REVIEW_STATES)
    {
        return Some(WaitDisposition::Actionable);
    }
    contains_any(text, PENDING_STATES).then_some(WaitDisposition::Nonterminal)
}

fn contains_any(text: &str, values: &[&str]) -> bool {
    values.iter().any(|value| text.contains(value))
}
