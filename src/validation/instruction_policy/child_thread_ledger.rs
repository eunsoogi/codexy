use std::path::Path;

use crate::paths::display_relative;

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if path.ends_with("skills/dreaming/SKILL.md") {
        require_all(
            path,
            text,
            errors,
            "dreaming skill must require a durable active/waiting child thread ledger",
            &[
                "in-progress/waiting child thread list",
                "issue/pr",
                "thread id",
                "status",
                "owner state",
                "blocker",
                "latest evidence",
                "next action",
                "removed from the ledger",
                "canonical worktree cwd",
                "frozen head",
                "clean/index state",
                "referencing specialist or sentinel",
                "explicit release/archive state",
                "must remain as worktree reservations",
                "must not recycle the worktree",
            ],
        );
    } else if path.ends_with("skills/codex-orchestration/SKILL.md") {
        require_all(
            path,
            text,
            errors,
            "orchestration skill must maintain the active child thread ledger",
            &[
                "active child codex app threads must be capped at 5",
                "existing issue/pr owner thread",
                "reuse it when present",
                "completed child threads must remain reserved",
                "unavailable archive/delete surface as unresolved reservation evidence",
                "blocked/rate-limited child lanes",
                "existing owner thread",
                "latest evidence",
                "compaction recovery",
                "normal polling",
                "packaged specialist subagents must not be counted",
                "canonical worktree cwd",
                "frozen head",
                "clean/index state",
                "explicit release/archive state",
                "must keep its reservation active",
                "must not silently recycle that worktree",
            ],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not allow specialist subagents to count against the child thread cap",
            &["packaged specialist subagents must not be counted unless"],
        );
    } else if path.ends_with("skills/codex-orchestration/references/thread-and-worktree-routing.md")
    {
        require_all(
            path,
            text,
            errors,
            "thread routing must require live worktree reservation preflight",
            &[
                "live worktree reservation preflight",
                "reservation map",
                "every active or waiting specialist or sentinel",
                "must not create or fork the new thread",
                "must fail setup before allocation",
                "host allocator blocker",
                "dirty or locked candidate worktrees",
            ],
        );
    }
}

fn require_all(
    path: &Path,
    text: &str,
    errors: &mut Vec<String>,
    requirement: &str,
    phrases: &[&str],
) {
    let lower = normalized_whitespace(text);
    for phrase in phrases {
        if !lower.contains(phrase) {
            errors.push(format!(
                "{} {requirement}: missing `{phrase}`",
                display_relative(path)
            ));
        }
    }
}

fn reject_all(
    path: &Path,
    text: &str,
    errors: &mut Vec<String>,
    requirement: &str,
    phrases: &[&str],
) {
    let lower = normalized_whitespace(text);
    for phrase in phrases {
        if lower.contains(phrase) {
            errors.push(format!(
                "{} {requirement}: forbidden `{phrase}`",
                display_relative(path)
            ));
        }
    }
}

fn normalized_whitespace(text: &str) -> String {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
