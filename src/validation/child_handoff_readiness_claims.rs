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
        let line = line.trim();
        if line.starts_with(['-', '*']) {
            return false;
        }
        let line = line.trim_end_matches('.');
        STANDALONE_READY_PHRASES.contains(&line) && !has_next_non_claim_bullet(&lines[index + 1..])
    })
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

fn has_next_non_claim_bullet(lines: &[&str]) -> bool {
    lines
        .iter()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
        .and_then(|line| line.strip_prefix(['-', '*']))
        .map(str::trim)
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
