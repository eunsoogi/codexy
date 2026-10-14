//! Canonical concern declarations shared by manifest and event-topology checks.

const PREVENTIVE_EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

pub(super) const EVENT_ORDER: &[&str] = &[
    "PermissionRequest",
    "PreToolUse",
    "PostToolUse",
    "Interrupt",
    "UserPromptSubmit",
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
        trigger: "^(?:(?:agents|multi_agent_v1)__)?spawn_agent$",
        input: "codexy.hooks.subagent-ownership.v1",
        launcher: "codexy-subagent-ownership",
        diagnostic: "CODEXY_SUBAGENT_OWNERSHIP_",
        events: PREVENTIVE_EVENTS,
        preventive: true,
    },
    Expected {
        id: "watcher-wait-interruption",
        trigger: "^(?:mcp__[^ ]+__)?watcher_wait$",
        input: "codexy.hooks.watcher-wait-interruption.v2",
        launcher: "codexy-watcher-interrupt",
        diagnostic: "CODEXY_WATCHER_INTERRUPT_",
        events: &["PreToolUse", "Interrupt", "UserPromptSubmit"],
        preventive: false,
    },
    Expected {
        id: "watcher-message-cancellation",
        trigger: "^mcp__codex_app__send_message_to_thread$",
        input: "codexy.hooks.watcher-message-cancellation.v1",
        launcher: "codexy-watcher-interrupt",
        diagnostic: "CODEXY_WATCHER_MESSAGE_CANCELLATION_",
        events: &["PostToolUse"],
        preventive: false,
    },
];
