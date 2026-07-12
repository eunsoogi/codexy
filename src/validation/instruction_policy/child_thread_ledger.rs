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
            false,
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
                "event-driven refresh",
                "packaged specialist subagents must not be counted",
                "canonical worktree cwd",
                "frozen head",
                "clean/index state",
                "explicit release/archive state",
                "must keep its reservation active",
                "must not silently recycle that worktree",
                "short-lived child implementation goal",
                "must not retain a persistent long-running goal",
                "must not autonomously poll",
                "exactly one terminal unavailable report",
                "must not retry the parent message",
                "no full conversation transfer",
                "no full agent-tree listing",
                "root/orchestrator may end its goal and plan after dispatch",
                "child external-gate wait must retain active goal and plan",
                "bounded child-local monitoring",
                "send a parent delta before transition",
                "inspect archive candidates and the active reservation ledger",
                "may archive only terminal, unreferenced, clean and unreserved worktree lanes with no open pr or pending gate",
                "must not archive pr owners or dirty/reserved candidates",
                "record the decision in setup evidence",
                "usable existing owner must record the `block` and update the plan to a repair step",
                "add faithful red coverage, repair, rerun terminal proof, then invoke exactly one fresh sentinel review for the new file state or head",
            ],
            false,
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not allow specialist subagents to count against the child thread cap",
            &["packaged specialist subagents must not be counted unless"],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must reject legacy persistent root goals and autonomous polling",
            &["must keep polling and keep the goal active"],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not block a usable owner goal after Sentinel BLOCK",
            &["must call update_goal(status=\"blocked\") after a sentinel block"],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not replace a usable owner after Sentinel BLOCK",
            &["must create a replacement thread after a sentinel block"],
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
            true,
        );
    }
}

fn require_all(
    path: &Path,
    text: &str,
    errors: &mut Vec<String>,
    requirement: &str,
    phrases: &[&str],
    allow_heading_matches: bool,
) {
    let lower = normalized_whitespace(text);
    for phrase in phrases {
        if !has_unweakened_required_clause(&lower, phrase, allow_heading_matches) {
            errors.push(format!(
                "{} {requirement}: missing `{phrase}`",
                display_relative(path)
            ));
        }
    }
}

fn has_unweakened_required_clause(text: &str, phrase: &str, allow_heading_matches: bool) -> bool {
    text.match_indices(phrase).any(|(index, _)| {
        let before = &text[..index];
        let after = text[index + phrase.len()..]
            .trim_start_matches([',', ':', ';', '-', '—'])
            .trim_start();
        (allow_heading_matches || !appears_in_heading(before))
            && !has_invalid_prefix(before)
            && !has_invalid_suffix(after)
    })
}

fn has_invalid_prefix(before: &str) -> bool {
    let section = before
        .rsplit("<markdown-heading>")
        .next()
        .unwrap_or_default();
    let clause = clause_prefix(section);
    section
        .trim_start()
        .starts_with("historical example </markdown-heading>")
        || clause.contains("historical example")
        || clause.contains("false that")
        || clause.starts_with("not required")
        || clause.starts_with("no longer required")
        || clause.trim_end().ends_with("it is not required that")
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
    ["unless ", "except ", "only if ", "may ", "is not required"]
        .iter()
        .any(|marker| after.starts_with(marker))
}

fn appears_in_heading(before: &str) -> bool {
    before.rfind("<markdown-heading>") > before.rfind("</markdown-heading>")
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
        if lower.match_indices(phrase).any(|(index, _)| {
            let before = &lower[..index];
            !appears_in_heading(before) && !has_invalid_prefix(before)
        }) {
            errors.push(format!(
                "{} {requirement}: forbidden `{phrase}`",
                display_relative(path)
            ));
        }
    }
}

fn normalized_whitespace(text: &str) -> String {
    let mut with_heading_boundaries = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            with_heading_boundaries.push_str(" <markdown-heading> ");
            with_heading_boundaries.push_str(trimmed.trim_start_matches('#').trim());
            with_heading_boundaries.push_str(" </markdown-heading> ");
        } else {
            with_heading_boundaries.push_str(line);
            with_heading_boundaries.push(' ');
        }
    }
    with_heading_boundaries
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
