use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

use super::command;
use super::context;

pub(super) const PURPOSE_PR_TITLE_CHECK: u8 = 1 << 2;
pub(super) const PURPOSE_PR_LABEL_CHECK: u8 = 1 << 3;
pub(super) const PURPOSE_MERGE_MESSAGE_CHECK: u8 = 1 << 4;

const READINESS_EVENT: &str = "UserPromptSubmit";
pub(super) const PR_TITLE_SCRIPT: &str = "hooks/codexy-pr-title-check.sh";
pub(super) const PR_LABEL_SCRIPT: &str = "hooks/codexy-pr-label-check.sh";
pub(super) const MERGE_MESSAGE_SCRIPT: &str = "hooks/codexy-merge-message-check.sh";

const HARD_HOOKS: &[HardHook] = &[
    HardHook {
        script: PR_TITLE_SCRIPT,
        purpose: PURPOSE_PR_TITLE_CHECK,
        fragments: &[
            "hard PR title check",
            "codexy-pr-title-check.sh --pr-title",
            "context-only hooks do not enforce",
        ],
    },
    HardHook {
        script: PR_LABEL_SCRIPT,
        purpose: PURPOSE_PR_LABEL_CHECK,
        fragments: &[
            "hard PR label check",
            "codexy-pr-label-check.sh --pr-state-file",
            "repositoryLabels",
            "context-only hooks do not enforce",
        ],
    },
    HardHook {
        script: MERGE_MESSAGE_SCRIPT,
        purpose: PURPOSE_MERGE_MESSAGE_CHECK,
        fragments: &[
            "hard merge-message check",
            "codexy-merge-message-check.sh --expected-pr",
            "context-only hooks do not enforce",
        ],
    },
];
const DELEGATED_HARD_HELPERS: &[&str] = &[
    "hooks/codexy-readiness-guard.sh",
    "hooks/codexy-readiness-guard-pr-labels.sh",
];

struct HardHook {
    script: &'static str,
    purpose: u8,
    fragments: &'static [&'static str],
}

pub(super) fn check_hard_hook(
    path: &Path,
    plugin_root: &Path,
    event: &str,
    command_text: &str,
    timeout_secs: u64,
) -> Result<u8> {
    if event != READINESS_EVENT {
        return Ok(0);
    }
    for hard_hook in HARD_HOOKS {
        if command_uses_script(command_text, hard_hook.script) {
            check_hard_hook_context(path, plugin_root, command_text, timeout_secs, hard_hook)?;
            return Ok(hard_hook.purpose);
        }
    }
    Ok(0)
}

pub(super) fn missing_hard_hook_message(missing_purpose: u8) -> Option<&'static str> {
    match missing_purpose {
        PURPOSE_PR_TITLE_CHECK => {
            Some("UserPromptSubmit hook command must run hooks/codexy-pr-title-check.sh")
        }
        PURPOSE_PR_LABEL_CHECK => {
            Some("UserPromptSubmit hook command must run hooks/codexy-pr-label-check.sh")
        }
        PURPOSE_MERGE_MESSAGE_CHECK => {
            Some("UserPromptSubmit hook command must run hooks/codexy-merge-message-check.sh")
        }
        _ => None,
    }
}

fn check_hard_hook_context(
    path: &Path,
    plugin_root: &Path,
    command_text: &str,
    timeout_secs: u64,
    hard_hook: &HardHook,
) -> Result<()> {
    let (hook_path, arguments) = command::plugin_root_entrypoint_path(command_text).with_context(
        || {
            format!(
                "{} {READINESS_EVENT} hook command must start with a packaged ${{PLUGIN_ROOT}} entrypoint",
                display_relative(path)
            )
        },
    )?;
    if !arguments
        .split_ascii_whitespace()
        .eq(std::iter::once(READINESS_EVENT))
    {
        bail!(
            "{} {READINESS_EVENT} hook command must invoke {READINESS_EVENT} exactly",
            display_relative(path)
        );
    }
    let script_path = plugin_root.join(&hook_path);
    let context =
        context::emitted_session_start_context(&script_path, READINESS_EVENT, timeout_secs)?;
    for fragment in hard_hook.fragments {
        if !context.contains(fragment) {
            bail!(
                "{} {READINESS_EVENT} emitted hard hook context must include {fragment:?}: {}",
                display_relative(path),
                display_relative(&script_path)
            );
        }
    }
    for helper in DELEGATED_HARD_HELPERS {
        command::check_script_safety(path, READINESS_EVENT, &plugin_root.join(helper))?;
    }
    Ok(())
}

fn command_uses_script(command: &str, script: &str) -> bool {
    let Some((hook_path, _)) = command::plugin_root_entrypoint_path(command) else {
        return false;
    };
    hook_path == Path::new(script)
}
