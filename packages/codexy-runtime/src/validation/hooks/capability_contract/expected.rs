//! Canonical concern declarations shared by manifest and event-topology checks.

const PREVENTIVE_EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

pub(super) const EVENT_ORDER: &[&str] = &[
    "PermissionRequest",
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "Interrupt",
];

pub(super) struct Expected {
    pub(super) id: &'static str,
    pub(super) trigger: &'static str,
    pub(super) input: &'static str,
    pub(super) launcher: &'static str,
    pub(super) diagnostic: &'static str,
    pub(super) events: &'static [&'static str],
    pub(super) preventive: bool,
}

// This ordered table drives schema checks, hook topology, entrypoints, and digests.
pub(super) const CONCERNS: &[Expected] = &[
    Expected {
        id: "thread-delivery",
        trigger: "^(?:codex_app__|mcp__codex_app__)send_message_to_thread$",
        input: "codexy.hooks.thread-delivery.v2",
        launcher: "codexy-thread-delivery",
        diagnostic: "CODEXY_THREAD_DELIVERY_",
        events: PREVENTIVE_EVENTS,
        preventive: true,
    },
    Expected {
        id: "child-thread-creation",
        trigger: "^(?:codex_app__|mcp__codex_app__)create_thread$",
        input: "codexy.hooks.child-thread-creation.v1",
        launcher: "codexy-child-thread-creation",
        diagnostic: "CODEXY_CHILD_THREAD_CREATION_",
        events: PREVENTIVE_EVENTS,
        preventive: true,
    },
    Expected {
        id: "subagent-ownership",
        trigger: "^(?:(?:agents|multi_agent_v1)__)?(?:spawn_agent|send_message|followup_task)$",
        input: "codexy.hooks.subagent-ownership.v1",
        launcher: "codexy-subagent-ownership",
        diagnostic: "CODEXY_SUBAGENT_OWNERSHIP_",
        events: PREVENTIVE_EVENTS,
        preventive: true,
    },
    Expected {
        id: "cleanup-progress",
        trigger: "^(?:codex_app__|mcp__codex_app__)(?:set_thread_archived|list_archived_threads)$",
        input: "codexy.hooks.cleanup-progress.v1",
        launcher: "codexy-cleanup-progress",
        diagnostic: "CODEXY_CLEANUP_PROGRESS_",
        events: &["PostToolUse", "Stop", "Interrupt"],
        preventive: false,
    },
];
