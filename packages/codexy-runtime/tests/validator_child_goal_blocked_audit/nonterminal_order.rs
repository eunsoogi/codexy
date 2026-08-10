use super::{CLASSIFICATION, TestResult, run_validator};

const WAIT: &str = "Nonterminal wait handoff: state fingerprint=sentinel-417-running; producer state=sentinel-running; wake route=sentinel-event; ownership=retained; goal state=active; plan state=active; goal transition=none; return control=confirmed\n";
const GOAL_CONTEXT: &str = "Source thread id: parent-417\nGoal control state: source_thread_id=parent-417\n";

pub(super) fn assert_boundaries() -> TestResult {
    for call in ["update_goal(complete)", "update_goal(status=\"complete\")", "update_goal(blocked)", "update_goal(status=\"blocked\")"] {
        assert_rejected(&format!("{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}evidence detail: unchanged wait\n{}", transition(call)))?;
        assert_rejected(&format!("{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{}Sentinel result: PASS\n", transition(call)))?;
    }
    for result in ["PASS", "BLOCK", "UNOBSERVABLE"] {
        for call in ["update_goal(complete)", "update_goal(blocked)"] {
            let output = run_validator(&format!("{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}Sentinel result: {result}\n{}", transition(call)))?;
            assert!(output.status.success(), "terminal {result} recovery rejected: {}", String::from_utf8_lossy(&output.stderr));
        }
    }
    for result in [
        "Reviewer gate: BLOCK",
        "Reviewer gate returned BLOCK",
        "Reviewer-gate verdict: BLOCK",
    ] {
        for call in ["update_goal(complete)", "update_goal(blocked)"] {
            assert_rejected(&format!(
                "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{result}\n{}",
                transition(call)
            ))?;
        }
    }
    for prefix in ["- [x] ", "+ [X] ", "1. [x] ", "- + [X] "] {
        for call in ["update_goal(complete)", "update_goal(blocked)"] {
            let transition = transition(call).replace(
                "Goal tool call:",
                &format!("{prefix}Goal tool call:"),
            );
            assert_rejected(&format!("{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{transition}"))?;
        }
        let output = run_validator(&format!(
            "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{prefix}Sentinel result: PASS\n{}",
            transition("update_goal(complete)")
        ))?;
        assert!(output.status.success(), "checked terminal result rejected: {}", String::from_utf8_lossy(&output.stderr));
    }
    for prefix in ["- [ ] ", "+ [ ] ", "1. [ ] ", "- + [ ] "] {
        let output = run_validator(&format!(
            "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{prefix}Goal tool call: update_goal(complete)\nSentinel result: PASS\n{}",
            transition("update_goal(complete)")
        ))?;
        assert!(output.status.success(), "unchecked task item entered lifecycle: {}", String::from_utf8_lossy(&output.stderr));
    }
    let checked_direction = transition("update_goal(blocked)").replace(
        "Blocked goal pre-mutation check:",
        "- [X] Parent direction event: version=direction-2; cancellation=received\nBlocked goal pre-mutation check:",
    );
    assert_invalid(&format!(
        "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}Sentinel result: BLOCK\n{checked_direction}"
    ), "cancelled by newer parent direction")?;
    for inactive in inactive_contexts() {
        for result in ["PASS", "BLOCK", "UNOBSERVABLE"] {
            for call in ["update_goal(complete)", "update_goal(blocked)"] {
                let result = inactive.replace("{event}", &format!("Sentinel result: {result}"));
                assert_rejected(&format!(
                    "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{result}## Current evidence\n{}",
                    transition(call)
                ))?;
            }
        }
    }
    for inactive in checked_task_inactive_contexts() {
        for result in ["PASS", "BLOCK", "UNOBSERVABLE"] {
            for call in ["update_goal(complete)", "update_goal(blocked)"] {
                let result = inactive.replace("{event}", &format!("Sentinel result: {result}"));
                assert_rejected(&format!(
                    "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{result}## Current evidence\n{}",
                    transition(call)
                ))?;
            }
        }
        let ignored_gate = inactive.replace("{event}", "Reviewer gate: BLOCK");
        let output = run_validator(&format!(
            "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{ignored_gate}## Current evidence\nSentinel result: PASS\n{}",
            transition("update_goal(complete)")
        ))?;
        assert!(output.status.success(), "inactive reviewer gate vetoed recovery: {}", String::from_utf8_lossy(&output.stderr));
    }
    for inactive in inactive_contexts() {
        let ignored_call = inactive.replace("{event}", "Goal tool call: update_goal(complete)");
        let output = run_validator(&format!(
            "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}{ignored_call}## Current evidence\nSentinel result: PASS\n{}",
            transition("update_goal(complete)")
        ))?;
        assert!(output.status.success(), "inactive goal call rejected recovery: {}", String::from_utf8_lossy(&output.stderr));

        let ignored_direction = inactive.replace(
            "{event}",
            "Parent direction event: version=direction-2; cancellation=received",
        );
        let transition = transition("update_goal(blocked)").replace(
            "Blocked goal pre-mutation check:",
            &format!("{ignored_direction}## Current evidence\nBlocked goal pre-mutation check:"),
        );
        let output = run_validator(&format!(
            "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}Sentinel result: BLOCK\n{transition}"
        ))?;
        assert!(output.status.success(), "inactive parent direction rejected recovery: {}", String::from_utf8_lossy(&output.stderr));
    }
    for evidence in [
        format!("{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}"),
        format!("{CLASSIFICATION}{GOAL_CONTEXT}{}{}", transition("update_goal(complete)"), WAIT),
        format!(
            "{CLASSIFICATION}{GOAL_CONTEXT}{WAIT}Lane ownership: parent-owned\nReviewer gate: BLOCK\nParent direction event: version=parent-2; cancellation=received\n{}",
            transition("update_goal(complete)")
        ),
    ] {
        let output = run_validator(&evidence)?;
        assert!(output.status.success(), "history or unrelated lane rejected: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn inactive_contexts() -> [&'static str; 5] {
    [
        "```text\n{event}\n```\n",
        "> {event}\n",
        "## Historical example\n{event}\n",
        "- > {event}\n",
        "- ```text\n- {event}\n- ```\n",
    ]
}

fn checked_task_inactive_contexts() -> [&'static str; 4] {
    [
        "- [x] > {event}\n",
        "- [x] ```text\n- [x] {event}\n- [x] ```\n",
        "- [x] ## Historical example\n- [x] {event}\n",
        "- [x] - [x] > {event}\n",
    ]
}

fn transition(call: &str) -> String {
    let (key, result, audit) = if call.contains("blocked") {
        ("417:blocked:wait", "blocked", "Blocked goal user-decision gate: gate id=wait-417; blocker class=user-decision; decision owner=user; user question=Should the migration preserve identifiers?; user response=unanswered; decision branches=preserve existing persisted identifiers|replace identifiers during migration; material impact=the choice changes persisted identifiers; safe default=unavailable; in-scope action=unavailable\n")
    } else { ("417:complete:wait", "complete", "") };
    let check = call.contains("blocked").then_some("Blocked goal pre-mutation check: gate id=wait-417; pre-delivery parent direction version=direction-1; current parent direction version=direction-1; cancellation=absent\n").unwrap_or_default();
    format!("Goal transition key: {key}\n{audit}Parent goal pre-delivery: operation={call}; parent task=parent-417; delivery=confirmed; task surface=codex task/thread; issue=#417; plan step=terminal; branch=codexy/417; worktree=/worktree; head=abc; clean/index=clean; evidence=terminal proof; next action=transition; parent direction version=direction-1; transition key={key}\nTerminal parent handoff: event id=terminal-child|417|review; issue/pr=#417; child task=child-417; parent task=parent-417; branch=codexy/417; worktree=/worktree; head=abc; clean/index=clean; last proof=terminal reviewer result; current gate=review terminal; preserved reservation/artifacts=worktree reserved; parent next action=continue goal transition; delivery=confirmed; task surface=codex task/thread\n{check}Goal tool call: {call}\nParent goal post-result: operation={call}; exact tool result={result}; parent task=parent-417; delivery=confirmed; task surface=codex task/thread; transition key={key}\n")
}

fn assert_rejected(evidence: &str) -> TestResult {
    assert_invalid(evidence, "nonterminal wait handoff")
}

fn assert_invalid(evidence: &str, expected: &str) -> TestResult {
    let output = run_validator(evidence)?;
    assert!(
        !output.status.success(),
        "terminal goal call before result passed: {evidence}"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains(expected), "{}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}
