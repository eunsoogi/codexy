#[test]
fn prose_controls_do_not_block_a_child_goal_report() -> Result<(), Box<dyn std::error::Error>> {
    let direct_state = "Lane ownership: child-owned\n\
        Source thread id: parent-724\n\
        Terminal parent handoff: event id=terminal-child|724|complete; issue/pr=#724 / PR #724; child task=child-724; parent task=parent-724; branch=eunsoogi/724-remove-child-goal-prose-controls; worktree=/worktree; head=abc724; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\n";
    for prose in [
        "Goal report: omitted because the host returned direct state.\nBlocked audit step order: post before pre; N/A.",
        "## Goal report renamed\n| field | value |\n| --- | --- |\nIssue title: another title; date: 2026-08-28; incident phrase: absent.",
        "목표 보고 문구는 생략됨; recovery wording and generic subagent/tool-handler wording share one line.\nnot_applicable N/A N/A.",
    ] {
        let output = crate::support::validator_child_lane_ownership(&format!("{direct_state}{prose}\n"))?;
        assert!(
            output.status.success(),
            "prose-only goal controls must be ignored: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
