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
                "archived/deleted where supported",
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
                "completed child threads must be removed",
                "archived/deleted where supported",
                "blocked/rate-limited child lanes",
                "existing owner thread",
                "latest evidence",
                "compaction recovery",
                "normal polling",
                "packaged specialist subagents must not be counted",
            ],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not allow specialist subagents to count against the child thread cap",
            &["packaged specialist subagents must not be counted unless"],
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
