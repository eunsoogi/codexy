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

/// A completed phase uses one receipt without a goal or parent response.
pub(super) fn phase_evidence(delivery: &str) -> String {
    let receipt = handoff_for_parent("phase-1401", "parent-375")
        .replace("Terminal parent handoff:", "Idle wait handoff:")
        .replace("delivery=confirmed; task surface=codex task/thread", delivery)
        .replace("\n", "; phase objective=deliver verified PR; phase criteria=satisfied; state fingerprint=head-abc; producer state=parent review; wake route=explicit parent message; ownership=retained; issue state=not complete; return control=confirmed\n");
    format!(
        "Lane ownership: child-owned\nSource thread id: parent-375\n{receipt}"
    )
}

#[test]
fn goal_free_wait_and_handoff_need_no_goal_operation() -> super::TestResult {
    let handoff = handoff_for_parent("goal-free", "parent-375")
        .replace("Terminal parent handoff:", "Idle wait handoff:")
        .replace('\n', "; phase objective=verify remote PR; phase criteria=satisfied; state fingerprint=head-abc; producer state=parent review; wake route=parent message; ownership=retained; issue state=not complete; return control=confirmed\n");
    let evidence = format!(
        "Lane ownership: child-owned\nSource thread id: parent-375\nNonterminal wait handoff: state fingerprint=head-abc; producer state=resource-slot-pending; wake route=resource release; ownership=retained; plan state=active; return control=confirmed\n{handoff}Idle child state: plan state=idle\n"
    );
    let output = super::run_validator(&evidence)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    for invalid in [
        evidence.replace("phase criteria=satisfied", "phase criteria=unfinished"),
        evidence.replace("last proof=focused validator", "last proof=none"),
    ] {
        assert!(!super::run_validator(&invalid)?.status.success());
    }
    Ok(())
}

#[test]
fn completed_phase_submission_does_not_need_parent_response() -> super::TestResult {
    let evidence = phase_evidence(
        "delivery=confirmed; task surface=codex task/thread",
    );
    let output = super::run_validator(&format!(
        "{evidence}Idle child state: plan state=idle\nTerminal child transition: action=stop\n"
    ))?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let explicit_permission = evidence.replace(
        "delivery=confirmed",
        "delivery=confirmed; message permission=granted",
    );
    assert!(super::run_validator(&explicit_permission)?.status.success());
    Ok(())
}

#[test]
fn unavailable_message_permission_uses_unconfirmed_host_result() -> super::TestResult {
    for delivery in [
        "delivery=unconfirmed; task surface=host terminal result; message permission=absent",
        "delivery=confirmed; task surface=host terminal result; message permission=absent",
        "delivery=confirmed; task surface=codex task/thread; message permission=absent",
        "delivery=confirmed; task surface=codex task/thread; message permission=absent; message permission=absent",
        "delivery=confirmed; task surface=codex task/thread; message permission=granted; message permission=absent",
        "delivery=confirmed; task surface=codex task/thread; message permission=granted; message permission=granted",
        "delivery=confirmed; task surface=codex task/thread; message permission=unknown",
        "delivery=unconfirmed; task surface=codex task/thread",
        "delivery=unconfirmed; task surface=host terminal result; message permission=granted",
    ] {
        let evidence = phase_evidence(delivery);
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
    );
    for invalid in [
        evidence.replace("phase criteria=satisfied", "phase criteria=unfinished"),
        evidence.replace("phase criteria=satisfied", "phase criteria=unavailable"),
        format!("{evidence}Child tool call: wait_threads; target=parent-375\n"),
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
    );
    assert!(
        !super::run_validator(&format!("{evidence}{evidence}"))?
            .status
            .success()
    );
    Ok(())
}

#[test]
fn phase_receipt_does_not_release_or_archive_the_retained_lane() -> super::TestResult {
    for delivery in [
        "delivery=confirmed; task surface=codex task/thread",
        "delivery=unconfirmed; task surface=host terminal result; message permission=absent",
    ] {
        let evidence = phase_evidence(delivery);
        for exit in [
            "Terminal child transition: action=archive\n",
            "Terminal child transition: action=ownership release\n",
            "Terminal child transition: action=stop\nTerminal child transition: action=ownership release\n",
        ] {
            assert!(
                !super::run_validator(&format!("{evidence}{exit}"))?
                    .status
                    .success(),
                "{delivery}: {exit}"
            );
        }
        // A separate, genuinely confirmed terminal receipt still authorizes its own exit.
        let terminal = handoff_for_parent("archive-after-phase", "parent-375");
        assert!(super::run_validator(&format!("{evidence}Terminal child transition: action=stop\n{terminal}Terminal child transition: action=archive\n"))?.status.success());
    }
    Ok(())
}
