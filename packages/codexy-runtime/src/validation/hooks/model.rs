use anyhow::{Result, bail};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HookPurpose {
    Admission,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct HookBinding {
    pub(super) event: HookEvent,
    pub(super) purpose: HookPurpose,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum HookEvent {
    PermissionRequest,
    PostCompact,
    PostToolUse,
    PreCompact,
    PreToolUse,
    SessionStart,
    Stop,
    SubagentStart,
    SubagentStop,
    UserPromptSubmit,
}

impl HookEvent {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::PermissionRequest => "PermissionRequest",
            Self::PostCompact => "PostCompact",
            Self::PostToolUse => "PostToolUse",
            Self::PreCompact => "PreCompact",
            Self::PreToolUse => "PreToolUse",
            Self::SessionStart => "SessionStart",
            Self::Stop => "Stop",
            Self::SubagentStart => "SubagentStart",
            Self::SubagentStop => "SubagentStop",
            Self::UserPromptSubmit => "UserPromptSubmit",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self> {
        match value {
            "PermissionRequest" => Ok(Self::PermissionRequest),
            "PostCompact" => Ok(Self::PostCompact),
            "PostToolUse" => Ok(Self::PostToolUse),
            "PreCompact" => Ok(Self::PreCompact),
            "PreToolUse" => Ok(Self::PreToolUse),
            "SessionStart" => Ok(Self::SessionStart),
            "Stop" => Ok(Self::Stop),
            "SubagentStart" => Ok(Self::SubagentStart),
            "SubagentStop" => Ok(Self::SubagentStop),
            "UserPromptSubmit" => Ok(Self::UserPromptSubmit),
            _ => bail!("unsupported hook event: {value}"),
        }
    }
}
