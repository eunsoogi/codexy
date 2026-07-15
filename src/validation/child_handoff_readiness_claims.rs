pub(super) fn child_readiness(text: &str) -> bool {
    has_any_affirmed(
        text,
        &[
            "child handoff",
            "parent handoff",
            "parent-handoff-ready",
            "pr ready for parent handoff",
            "parent can open pr next: yes",
            "parent can merge",
            "remote/pr head match: yes",
            "pushed: yes",
            "branch clean",
            "clean, synced",
            "synced, and pushed",
        ],
    )
}

pub(super) fn clean(text: &str) -> bool {
    has_any_affirmed(
        text,
        &[
            "branch clean",
            "worktree clean",
            "branch is clean",
            "worktree is clean",
            "clean: yes",
            "clean yes",
            "branch clean: yes",
            "worktree clean: yes",
            "dirty state: clean",
            " clean,",
        ],
    )
}

pub(super) fn synced(text: &str) -> bool {
    has_any_affirmed(text, &["synced", "remote/pr head match: yes"])
}

pub(super) fn pushed(text: &str) -> bool {
    has_any_affirmed(text, &["pushed", "remote/pr head match: yes"])
        && !super::child_handoff_readiness_text::has_non_claim_phrase_label(text, "pushed branch")
}

pub(super) fn pr_ready(text: &str) -> bool {
    has_any_affirmed(
        text,
        &[
            "pr ready",
            "pr-ready",
            "pr is ready",
            "pull-request-ready",
            "pull request ready",
            "pull request is ready",
            "pr readiness",
            "pr-readiness",
            "ready for parent handoff",
            "ready for handoff",
            "parent-handoff-ready",
            "parent handoff ready",
            "ready to merge",
            "ready for merge",
            "merge readiness",
            "merge-readiness",
            "merge-ready",
            "merge ready",
            "parent can open pr next",
            "parent can open pr next: yes",
            "parent can merge",
        ],
    )
}

pub(super) fn standalone_ready_line(text: &str) -> bool {
    let lines: Vec<_> = text.lines().collect();
    lines.iter().enumerate().any(|(index, line)| {
        let Some((line, is_bullet)) = standalone_line_text(line) else {
            return false;
        };
        let line = line.trim_end_matches('.');
        STANDALONE_READY_PHRASES.contains(&line)
            && (!is_bullet || explicit_bullet_ready_phrase(line))
            && !has_next_non_claim_bullet(&lines[index + 1..])
    })
}

pub(super) fn ready_label_phrases() -> &'static [&'static str] {
    STANDALONE_READY_PHRASES
}

fn has_any_affirmed(text: &str, phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}

const STANDALONE_READY_PHRASES: &[&str] = &[
    "pr ready",
    "pr-ready",
    "pr is ready",
    "pull-request-ready",
    "pull request ready",
    "pull request is ready",
    "pr readiness",
    "pr-readiness",
    "ready for parent handoff",
    "ready for handoff",
    "parent-handoff-ready",
    "parent handoff ready",
    "ready to merge",
    "ready for merge",
    "merge readiness",
    "merge-readiness",
    "merge-ready",
    "merge ready",
    "parent can open pr next",
    "parent can merge",
];

fn standalone_line_text(line: &str) -> Option<(&str, bool)> {
    let line = line.trim();
    let Some(bullet) = line
        .strip_prefix(['-', '*'])
        .or_else(|| strip_ordered_list_marker(line))
    else {
        return Some((line, false));
    };
    let bullet = bullet.trim_start();
    if bullet.starts_with("[ ]") {
        return None;
    }
    Some((
        bullet
            .strip_prefix("[x]")
            .or_else(|| bullet.strip_prefix("[X]"))
            .map(str::trim_start)
            .unwrap_or(bullet),
        true,
    ))
}

pub(super) fn strip_ordered_list_marker(line: &str) -> Option<&str> {
    let (number, rest) = line.split_once(['.', ')'])?;
    if number.is_empty() || !number.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    rest.strip_prefix(char::is_whitespace)
}

fn explicit_bullet_ready_phrase(line: &str) -> bool {
    line.contains("pr") || line.contains("merge") || line.contains("pull request")
}

pub(super) fn has_next_non_claim_bullet(lines: &[&str]) -> bool {
    lines
        .iter()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
        .and_then(|line| {
            line.strip_prefix(['-', '*', '+'])
                .or_else(|| strip_ordered_list_marker(line))
        })
        .map(str::trim)
        .map(|line| {
            line.strip_prefix("[ ]")
                .or_else(|| line.strip_prefix("[x]"))
                .or_else(|| line.strip_prefix("[X]"))
                .unwrap_or(line)
                .trim()
        })
        .is_some_and(|line| {
            if line.starts_with("no blockers") {
                return false;
            }
            [
                "not ", "not-", "not_", "n/a", "na", "none", "no", "missing", "absent",
            ]
            .iter()
            .any(|phrase| line.starts_with(phrase))
        })
}
