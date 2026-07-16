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
        "orchestration.external-wait.no-active-goal",
        "parent or child",
        Modality::Prohibited,
        &["retain"],
        &["active goal", "plan"],
    )
    .in_lifecycle(&["external-gate wait"]),
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
        "heartbeat.waiting.no-persistent-goal",
        "owner",
        Modality::Prohibited,
        &["retain", "recreate", "execution goal"],
        &[],
    )
    .in_lifecycle(&["registered heartbeat"]),
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
