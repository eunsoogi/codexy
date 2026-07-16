use std::path::Path;

use super::clauses::{reject_all, require_all};

const REQUIRED_CLAUSES: &[&str] = &[
    "Every non-trivial child lane MUST declare a finite execution budget before edits begin.",
    "The budget MUST name finite implementation, repair, and reviewer cycle limits.",
    "Continuation MUST consume budget and record either an explicit acceptance criterion newly satisfied or an existing blocker removed.",
    "File, diff, test, or fingerprint churn without reducing remaining acceptance work MUST NOT renew or reset the budget.",
    "A renewal MUST be an explicit parent-owned new finite budget with recorded acceptance progress or blocker removal.",
    "After all acceptance criteria and required proof are complete, the lane MUST terminate implementation; adjacent findings become non-blocking follow-up candidates.",
    "Budget exhaustion MUST produce one compact terminal parent handoff with current goal/plan, branch/worktree/HEAD, dirty inventory, proof, remaining criteria, and recommended next decision.",
    "Budget exhaustion MUST NOT call `update_goal(blocked)` and MUST NOT weaken external-gate heartbeat semantics.",
    "An external parent heartbeat MUST observe waiting state without messaging the child and MUST send one continuation only on a material transition.",
    "Repeated child waiting turns, goal refreshes, polling, duplicate narrative, unbounded reasoning, or status-only parent receipts MUST consume budget and MUST NOT qualify as acceptance progress.",
    "The execution-budget contract MUST apply to GPT-5.6 Terra child lanes while remaining model-agnostic and MUST NOT hard-code model-specific prose into the state machine.",
];
const COUNTERMANDING_CLAUSES: &[&str] = &[
    "Artifact churn MAY renew or reset the budget.",
    "A child MAY self-renew the budget from changed artifacts alone.",
    "Budget exhaustion MAY call `update_goal(blocked)`.",
    "Repeated child waiting turns, goal refreshes, or polling MAY qualify as acceptance progress.",
];

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if !path.ends_with("skills/codex-orchestration/references/execution-budget.md") {
        return;
    }
    require_all(
        path,
        text,
        errors,
        "execution-budget contract must preserve finite acceptance-based termination",
        REQUIRED_CLAUSES,
    );
    reject_all(
        path,
        text,
        errors,
        "execution-budget contract must reject countermanding churn, blocked-goal, and wait policy",
        COUNTERMANDING_CLAUSES,
    );
}
