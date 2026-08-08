pub(super) fn handoff_for_parent(event: &str, parent_task: &str) -> String {
    handoff_for_tasks(event, parent_task, "child-375")
}

pub(super) fn handoff_for_tasks(event: &str, parent_task: &str, child_task: &str) -> String {
    format!(
        "Terminal parent handoff: event id=terminal-child|375|{event}; issue/pr=#375 / PR #376; child task={child_task}; parent task={parent_task}; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\n"
    )
}
