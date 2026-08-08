#![allow(dead_code)]

use super::structured_contract::{Modality, Rule};

pub(crate) const ORCHESTRATION: &[Rule] = &[
    Rule::new(
        "orchestration.root.no-autonomous-polling",
        "root/orchestrator",
        Modality::Prohibited,
        &["autonomously", "poll"],
        &[],
    )
    .under_heading("event-driven token and quota containment"),
    Rule::new(
        "orchestration.external-wait.retain-active-goal",
        "parent or child",
        Modality::Required,
        &["retain"],
        &["active goal", "plan"],
    )
    .in_lifecycle(&["nonterminal external-gate wait"]),
];

pub(crate) const HEARTBEAT: &[Rule] = &[
    Rule::new(
        "heartbeat.register.thread-targeted",
        "creation",
        Modality::Required,
        &["use", "destination"],
        &["thread"],
    ),
    Rule::new(
        "heartbeat.waiting.retain-active-goal",
        "owner",
        Modality::Required,
        &["retain"],
        &["active goal", "plan"],
    )
    .in_lifecycle(&["implementation obligation"]),
    Rule::new(
        "heartbeat.sentinel.read-only",
        "owner",
        Modality::Prohibited,
        &["fold", "observation"],
        &["live packaged sentinel", "heartbeat"],
    ),
];

pub(crate) const TOKEN_CONTAINMENT: &[Rule] = &[
    Rule::new(
        "token.runtime-identity.heartbeat-bound",
        "heartbeat route",
        Modality::Required,
        &["bind", "observation"],
        &["automation id", "target thread", "bounded schedule"],
    ),
    Rule::new(
        "token.runtime-identity.no-process-resume",
        "heartbeat route",
        Modality::Prohibited,
        &["require"],
        &["persistent exec/session", "same-process resume"],
    ),
    Rule::new(
        "token.containment.no-autonomous-polling",
        "root/orchestrator",
        Modality::Prohibited,
        &["autonomously", "poll"],
        &[],
    )
    .under_heading("event-driven delta"),
];

pub(crate) const DELEGATION: &[Rule] = &[
    Rule::new(
        "delegation.helper.no-recursion",
        "agent",
        Modality::Prohibited,
        &["spawn", "delegate", "create"],
        &["helper", "reviewer", "task", "thread"],
    ),
    Rule::new(
        "delegation.child.first-level-only",
        "child implementation thread",
        Modality::Permitted,
        &["spawn"],
        &["first-level specialist helpers", "sentinel reviewers"],
    ),
    Rule::new(
        "delegation.assignment.nonrecursive",
        "helper or sentinel assignment",
        Modality::Required,
        &["include"],
        &["nonrecursive delegation prohibition"],
    ),
];

pub(crate) const TOKEN_PROMPT: &[Rule] = &[Rule::new(
    "token.prompt.required-invocation",
    "you",
    Modality::Required,
    &["use"],
    &["$token-efficient-orchestration", "event-driven handoffs"],
)];

pub(crate) const PARENT_EXECUTION_BUDGET: &[Rule] = &[
    Rule::new(
        "parent-budget.declared-before-work",
        "parent-owned orchestration stage",
        Modality::Required,
        &["declare"],
        &["finite implementation", "repair", "fanout", "reviewer-cycle limits"],
    ),
    Rule::new(
        "parent-budget.specialist-cap",
        "parent-owned stage",
        Modality::Prohibited,
        &["use"],
        &["more than three non-sentinel specialists"],
    ),
    Rule::new(
        "parent-budget.repeated-cycle-progress",
        "parent helper or reviewer cycle",
        Modality::Required,
        &["record"],
        &["acceptance criterion newly satisfied", "existing blocker removed"],
    ),
    Rule::new(
        "parent-budget.wait-replay-consumes-budget",
        "unchanged wait output and full-state replay",
        Modality::Required,
        &["consume"],
        &["parent-stage budget"],
    ),
    Rule::new(
        "parent-budget.fallback-bounded",
        "bounded thread-read fallback that returns oversized preview or history output",
        Modality::Required,
        &["consume"],
        &["parent-stage budget", "bounded size", "token metadata"],
    ),
    Rule::new(
        "parent-budget.heartbeat-and-sentinel",
        "parent-stage budget enforcement",
        Modality::Required,
        &["preserve"],
        &["external-wait heartbeat", "packaged sentinel review gate"],
    ),
];

pub(crate) const TRANSITION: &[Rule] = &[
    Rule::new(
        "transition.runtime-monitor.outside-goal",
        "runtime monitor",
        Modality::Required,
        &["live"],
        &["outside", "execution goal"],
    )
    .under_heading("runtime polling boundary"),
    Rule::new(
        "transition.continuation.no-reschedule",
        "unchanged continuation turns",
        Modality::Prohibited,
        &["reschedule", "emit"],
        &["another unchanged turn"],
    )
    .under_heading("runtime polling boundary"),
    Rule::new(
        "transition.delivery.before-exit",
        "delivery",
        Modality::Required,
        &["confirmed"],
        &["stop", "archive", "release"],
    )
    .under_heading("ordered receipts"),
];
