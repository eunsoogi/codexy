use std::path::Path;

use crate::paths::display_relative;

mod clauses;
mod markdown;
use clauses::{has_false_requirement, has_mutating_permission};
use markdown::normalized_whitespace;

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
                "event-driven observation",
                "status observation of a running packaged sentinel must be read-only",
                "must not send messages, interrupts, follow-up prompts, or other mutations",
                "a live sentinel must remain active until it produces its own `pass`, `block`, or `unobservable` terminal result",
                "delayed output alone must not cause `unobservable`",
                "parent policy must use event-driven terminal deltas and must not poll a running sentinel",
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
    } else if path.ends_with("skills/proof-driven-completion/SKILL.md") {
        require_all(
            path,
            text,
            errors,
            "proof-completion skill must preserve live Sentinel observation",
            &[
                "for a running packaged sentinel, parent observation must be read-only",
                "must not poll, send messages, interrupts, or follow-up prompts",
                "sentinel must remain active until its own `pass`, `block`, or `unobservable` terminal result",
            ],
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
        if !has_unweakened_required_clause(&lower, phrase) {
            errors.push(format!(
                "{} {requirement}: missing `{phrase}`",
                display_relative(path)
            ));
        }
    }
}

fn has_unweakened_required_clause(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(index, _)| {
        let before = &text[..index];
        let after = text[index + phrase.len()..]
            .trim_start_matches([',', ':', ';', '.', '-', '—'])
            .trim_start();
        !has_invalid_prefix(before) && !has_invalid_suffix(after)
    })
}

fn has_invalid_prefix(before: &str) -> bool {
    let section = before
        .rsplit("</markdown-heading>")
        .next()
        .unwrap_or_default();
    let clause = clause_prefix(section);
    before.rfind("<markdown-heading>") > before.rfind("</markdown-heading>")
        || has_invalid_context(clause)
        || has_invalid_context(most_recent_heading(before))
}

fn most_recent_heading(before: &str) -> &str {
    before
        .rsplit("<markdown-heading>")
        .next()
        .and_then(|heading_and_text| heading_and_text.split_once("</markdown-heading>"))
        .map(|(heading, _)| heading)
        .unwrap_or_default()
}

fn has_invalid_context(text: &str) -> bool {
    [
        "historical example",
        "stale example",
        "example only",
        "not required",
        "no longer required",
        "false that",
    ]
    .iter()
    .any(|marker| text.contains(marker))
        || has_false_requirement(text)
}

fn clause_prefix(section: &str) -> &str {
    let mut start = 0;
    for (index, character) in section.char_indices() {
        let after = section[index + character.len_utf8()..].trim_start();
        if character == ';'
            || (character == '.'
                && !after
                    .chars()
                    .next()
                    .is_some_and(|item| item.is_ascii_digit()))
        {
            start = index + character.len_utf8();
        }
    }
    &section[start..]
}

fn has_invalid_suffix(after: &str) -> bool {
    [
        "unless ",
        "except ",
        "only if ",
        "but ",
        "however ",
        "although ",
        "it is not required",
    ]
    .iter()
    .any(|marker| after.starts_with(marker))
        || has_mutating_permission(after)
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
