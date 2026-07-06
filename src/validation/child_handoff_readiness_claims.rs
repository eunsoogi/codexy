pub(super) fn child_readiness(text: &str) -> bool {
    has_any_affirmed(
        text,
        &[
            "child handoff",
            "parent handoff",
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
            "ready to merge",
            "ready for merge",
            "merge readiness",
            "merge-readiness",
            "merge-ready",
            "merge ready",
            "parent can open pr next: yes",
            "parent can merge",
        ],
    )
}

fn has_any_affirmed(text: &str, phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| super::child_handoff_readiness_text::has_affirmed_phrase(text, phrase))
}
