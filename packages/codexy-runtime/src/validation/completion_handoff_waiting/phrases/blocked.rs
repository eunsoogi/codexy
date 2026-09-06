pub(in super::super) const CURRENT_CLAIM: &[&str] = &[
    "now blocked",
    "currently blocked",
    "still blocked",
    "remains blocked",
    "is blocked",
    "goal blocked",
    "work blocked",
    "lane blocked",
];

pub(in super::super) const DISALLOWED_RATIONALE: &[&str] = &[
    "maintainer input",
    "human input",
    "external state change",
    "true impasse",
];

pub(in super::super) const USER_DECISION: &[&str] =
    &["unanswered user decision", "missing user information"];
pub(in super::super) const MATERIAL_CHANGE: &[&str] =
    &["materially changes the result", "materially changes result"];
pub(in super::super) const NO_SAFE_DEFAULT: &[&str] =
    &["no safe default", "safe default=unavailable"];
pub(in super::super) const NO_IN_SCOPE_ACTION: &[&str] =
    &["no in-scope action", "in-scope action=unavailable"];
pub(in super::super) const ACTIVE_BLOCKERS_OR_REMAINING: &[&str] =
    &["active blockers", "remaining"];
