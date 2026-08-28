#[test]
fn prose_controls_do_not_block_a_child_lane() -> Result<(), Box<dyn std::error::Error>> {
    let direct_state = "Lane ownership: child-owned\n\
        Source thread id: parent-724\n\
        Terminal parent handoff: event id=terminal-child|724|complete; issue/pr=#724 / PR #724; child task=child-724; parent task=parent-724; branch=eunsoogi/724-remove-child-goal-prose-controls; worktree=/worktree; head=abc724; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\n";
    for prose in [
        "PR: #724\nReview response: parent-authored implementation commit abc123 fixed feedback.\nMaintainer reassignment: none\nChild branch was created before formal orchestration evidence completed.",
        "# Renamed heading\n| Issue title | revised wording |\n| --- | --- |\nIssue title: unrelated wording; date: 2026-08-28; incident phrase: renamed.",
        "소유권 회복 문구: 없음; 검토 응답은 같은 줄의 subagent/tool-handler 표현이다.\nN/A not_applicable N/A.",
    ] {
        let output = crate::support::validator_child_lane_ownership(&format!("{direct_state}{prose}\n"))?;
        assert!(
            output.status.success(),
            "prose-only lane controls must be ignored: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
