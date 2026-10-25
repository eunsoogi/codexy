/// Builds a receipt using the fixture's standard child task identity.
pub(super) fn handoff_for_parent(event: &str, parent_task: &str) -> String {
    handoff_for_tasks(event, parent_task, "child-375")
}

/// Renders the required terminal receipt fields with caller-selected task identities.
pub(super) fn handoff_for_tasks(event: &str, parent_task: &str, child_task: &str) -> String {
    format!(
        "Terminal parent handoff: event id=terminal-child|375|{event}; issue/pr=#375 / PR #376; child task={child_task}; parent task={parent_task}; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\n"
    )
}

pub(super) fn terminal_goal_transition(status: &str, key: &str, include_handoff: bool) -> String {
    let operation = format!("update_goal(status=\"{status}\")");
    let transition_key = format!("375:{status}:{key}");
    let handoff =
        include_handoff.then(|| handoff_for_parent(&format!("{status}|{key}"), "parent-375"));
    let gate = (status == "blocked")
        .then(super::blocked_goal::gate)
        .unwrap_or_default();
    let parent_direction = (status == "blocked")
        .then_some("; parent direction version=direction-375")
        .unwrap_or_default();
    let pre_mutation = (status == "blocked")
        .then_some(super::blocked_goal::pre_mutation_check())
        .unwrap_or_default();
    format!(
        "Goal transition key: {transition_key}\n{gate}Parent goal pre-delivery: operation={operation}; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=proof; next action=complete{parent_direction}; transition key={transition_key}\n{pre_mutation}{}Goal tool call: {operation}\nParent goal post-result: operation={operation}; exact tool result={status}; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key={transition_key}\n",
        handoff.unwrap_or_default(),
    )
}

/// A completed phase uses one receipt and a local goal result, with no parent response.
fn phase_evidence(delivery: &str, status: &str) -> String {
    let receipt = handoff_for_parent("phase-1401", "parent-375")
        .replace("Terminal parent handoff:", "Idle wait handoff:")
        .replace("delivery=confirmed; task surface=codex task/thread", delivery)
        .replace("\n", "; phase objective=deliver verified PR; phase criteria=satisfied; state fingerprint=head-abc; producer state=parent review; wake route=explicit parent message; ownership=retained; issue state=not complete; goal transition=complete; return control=confirmed\n");
    format!(
        "Lane ownership: child-owned\nSource thread id: parent-375\n{receipt}Goal tool call: update_goal(complete)\nGoal tool result: operation=update_goal(complete); status={status}\n"
    )
}

#[test]
fn completed_phase_submission_does_not_need_parent_response() -> super::TestResult {
    let evidence = phase_evidence(
        "delivery=confirmed; task surface=codex task/thread",
        "complete",
    );
    let output = super::run_validator(&format!(
        "{evidence}Idle child state: goal state=complete; plan state=idle\nTerminal child transition: action=stop\n"
    ))?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status_form = evidence.replace("update_goal(complete)", "update_goal(status=\"complete\")");
    assert!(super::run_validator(&status_form)?.status.success());
    Ok(())
}

#[test]
fn failed_goal_transition_cannot_claim_idle_completion() -> super::TestResult {
    for status in ["active", "error", "unknown"] {
        let evidence = phase_evidence("delivery=confirmed; task surface=codex task/thread", status);
        assert!(
            super::run_validator(&evidence)?.status.success(),
            "an honest failed transition remains nonterminal"
        );
        assert!(
            !super::run_validator(&format!(
                "{evidence}Idle child state: goal state=complete; plan state=idle\n"
            ))?
            .status
            .success()
        );
    }
    Ok(())
}

#[test]
fn unavailable_message_permission_uses_unconfirmed_host_result() -> super::TestResult {
    for delivery in [
        "delivery=unconfirmed; task surface=host terminal result; message permission=absent",
        "delivery=confirmed; task surface=host terminal result; message permission=absent",
        "delivery=unconfirmed; task surface=codex task/thread",
        "delivery=unconfirmed; task surface=host terminal result; message permission=granted",
    ] {
        let evidence = phase_evidence(delivery, "complete");
        assert_eq!(
            super::run_validator(&evidence)?.status.success(),
            delivery.ends_with("permission=absent") && delivery.starts_with("delivery=unconfirmed"),
            "{delivery}"
        );
    }
    Ok(())
}

#[test]
fn phase_completion_rejects_unfinished_objectives_and_parent_waits() -> super::TestResult {
    let evidence = phase_evidence(
        "delivery=confirmed; task surface=codex task/thread",
        "complete",
    );
    for invalid in [
        evidence.replace("phase criteria=satisfied", "phase criteria=unfinished"),
        evidence.replace("phase criteria=satisfied", "phase criteria=unavailable"),
        format!("{evidence}Child tool call: wait_threads; target=parent-375\n"),
        evidence.replace("status=complete\n", "status=complete; status=active\n"),
        evidence.replace(
            "Goal tool call: update_goal(complete)",
            "Terminal child transition: action=stop",
        ),
        evidence.replace(
            "Goal tool result: operation=update_goal(complete); status=complete\n",
            "",
        ),
        evidence.replace(
            "Goal tool result: operation=update_goal(complete); status=complete\n",
            "Idle child state: goal state=complete; plan state=idle\n",
        ),
    ] {
        assert!(
            !super::run_validator(&invalid)?.status.success(),
            "{invalid}"
        );
    }
    Ok(())
}

#[test]
fn consumed_phase_event_cannot_authorize_another_completion() -> super::TestResult {
    let evidence = phase_evidence(
        "delivery=confirmed; task surface=codex task/thread",
        "complete",
    );
    assert!(
        !super::run_validator(&format!("{evidence}{evidence}"))?
            .status
            .success()
    );
    Ok(())
}
