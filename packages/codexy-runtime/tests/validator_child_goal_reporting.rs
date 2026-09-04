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

pub(super) fn clear_child_assignment(extra: &str) -> String {
    format!(
        "Ownership metadata source: parent-supplied\n\
         Lane ownership: child-owned\n\
         Task classification:\n\
         | Field | Value |\n\
         | --- | --- |\n\
         | Lane type | implementation |\n\
         | Secondary surfaces | validators |\n\
         | Owner decision | affirmative child-owned because the delegated child owns the work |\n\
         | Atomic scope | issue-sized |\n\
         | Required skills | orchestration, goal-lifecycle |\n\
         | Required tools/evidence | goal readback and focused validation |\n\
         | First allowed action | call get_goal before implementation |\n\
         | Stop/blocker | unrelated active goal |\n\
         Assignment: implement issue #873 with focused regression coverage and stop before merge.\n\
         Authorized goal objective: implement issue #873\n\
         {extra}\n"
    )
}

#[test]
fn rejects_goal_tool_prohibition_on_clear_child_implementation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(
        "Goal tools are not authorized unless the user explicitly requests a goal.",
    ))?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("clear delegated implementation must not prohibit available goal tools")
    );
    Ok(())
}

#[test]
fn clear_assignment_needs_no_second_goal_opt_in() -> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(
        "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":null}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get-null\n\
         Parent goal pre-delivery: operation=create_goal; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; issue=#873; plan step=implementation; branch=eunsoogi/873-finite-goals; worktree=/worktree; head=abc873; clean/index=clean; evidence=clear assignment; next action=create finite goal; transition key=873:create\n\
         Goal tool call: create_goal(objective=implement issue #873)\n\
         Parent goal post-result: operation=get_goal; exact tool result={\"goal\":{\"objective\":\"implement issue #873\",\"status\":\"active\"}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get-active",
    ))?;
    assert!(
        output.status.success(),
        "a concrete assignment is the authorization: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn ambiguous_discussion_does_not_create_a_goal_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::validator_child_lane_ownership(
        "Discussion note: perhaps issue #873 could be explored later.\n\
         Quoted anti-pattern: goal tools are not authorized unless a user requests a goal.\n",
    )?;
    assert!(output.status.success());
    Ok(())
}

#[test]
fn unrelated_active_goal_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(
        "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":{\"objective\":\"issue #999\",\"status\":\"active\"}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get\n\
         Goal tool call: create_goal(objective=issue #873)",
    ))?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("active goal must be preserved and must not be replaced by create_goal")
    );
    Ok(())
}

#[test]
fn rejects_equivalent_goal_tool_prohibitions() -> Result<(), Box<dyn std::error::Error>> {
    for prohibition in [
        "You are not authorized to use goal tools.",
        "Goal tools are **not authorized** for this task.",
        "The child must not use goal tools.",
        "Goal tools are disabled while implementation continues.",
    ] {
        let assignment = clear_child_assignment(prohibition)
            .replace("| Lane type | implementation |", "| Lane type | implementation and validation |");
        let output = crate::support::validator_child_lane_ownership(&assignment)?;
        assert!(
            !output.status.success(),
            "equivalent prohibition must fail: {prohibition}"
        );
    }
    Ok(())
}

#[test]
fn quoted_and_inert_prohibition_examples_are_ignored() -> Result<(), Box<dyn std::error::Error>> {
    for example in [
        "Quoted anti-pattern: Do not use goal tools.",
        "- \"Do not use goal tools\" is wording from the incident.",
        "> Do not use goal tools.",
        "```text\nDo not use goal tools.\n```",
    ] {
        let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(example))?;
        assert!(
            output.status.success(),
            "quoted or inert example must not become policy: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn active_goal_receipt_is_parsed_structurally() -> Result<(), Box<dyn std::error::Error>> {
    let active = clear_child_assignment(
        "Parent goal post-result: operation=get_goal; exact tool result={\"goal\": {\"objective\": \"issue #999\", \"status\" : \"active\"}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get\n\
         Parent goal pre-delivery: operation=create_goal; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:create",
    );
    let output = crate::support::validator_child_lane_ownership(&active)?;
    assert!(!output.status.success());

    let null_with_note = clear_child_assignment(
        "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":null,\"note\":\"status=active is not current\"}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get\n\
         Parent goal pre-delivery: operation=create_goal; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:create\n\
         Goal tool call: create_goal(objective=implement issue #873)\n\
         Parent goal post-result: operation=get_goal; exact tool result={\"goal\":{\"objective\":\"implement issue #873\",\"status\":\"active\"}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get-active",
    );
    let output = crate::support::validator_child_lane_ownership(&null_with_note)?;
    assert!(
        output.status.success(),
        "nested text must not spoof authoritative status: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn latest_get_goal_result_controls_creation() -> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::validator_child_lane_ownership(&clear_child_assignment(
        "Parent goal post-result: operation=get_goal; exact tool result={\"goal\":{\"status\":\"active\"}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get-active\n\
         Parent goal post-result: operation=get_goal; exact tool result={\"goal\":null}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get-cleared\n\
         Goal tool call: create_goal(objective=implement issue #873)\n\
         Parent goal post-result: operation=get_goal; exact tool result={\"goal\":{\"objective\":\"implement issue #873\",\"status\":\"active\"}}; parent task=parent-873; delivery=confirmed; task surface=codex task/thread; transition key=873:get-new-active",
    ))?;
    assert!(
        output.status.success(),
        "a later cleared readback permits creation: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
