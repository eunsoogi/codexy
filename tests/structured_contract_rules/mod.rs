#![allow(dead_code)]

use super::structured_contract::{Modality, Rule};

pub(crate) const ORCHESTRATION: &[Rule] = &[
    Rule::new(
        "orchestration.root.no-autonomous-polling",
        "root/orchestrator",
        Modality::Prohibited,
        &["autonomously", "poll"],
        &["terminal child state", "sentinel verdict"],
    ),
    Rule::new(
        "orchestration.external-wait.no-active-goal",
        "parent or child",
        Modality::Prohibited,
        &["retain", "active goal", "plan"],
        &["external-gate wait"],
    ),
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
        &["registered heartbeat"],
    ),
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
        &["qualifying events"],
    ),
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
