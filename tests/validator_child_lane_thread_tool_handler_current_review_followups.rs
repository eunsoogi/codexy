use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

fn base_evidence(route: &str, issue: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{route}
{issue}
Maintainer reassignment: none
"#
    )
}

#[test]
fn validator_rejects_negated_follow_up_issue_claim() -> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "No follow-up issue #205",
        "No separate follow-up issue #205",
        "Not a follow-up issue #205",
        "Follow-up issue: issue wasn't yet created for #205",
        "Follow-up issue: issue hasn't yet been created for #205",
        "Follow-up issue: issue had not been created for #205",
        "Follow-up issue: issue hadn't been created for #205",
        "Follow-up issue: issue hadn't yet been created for #205",
        "Follow-up issue: not filed for #205",
        "Follow-up issue: issue wasn't filed for #205",
        "Follow-up issue: issue has not been filed for #205",
        "Follow-up issue: issue hasn't been filed for #205",
    ] {
        let output = run_ownership_validator(&base_evidence(
            "Fallback route: no fallback route was available",
            issue,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject negated follow-up issue claims: {issue}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_long_negated_fallback_value() -> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: not used because the child thread was unreachable",
        "Fallback route: was not used because the child thread was unreachable",
        "Fallback route: not actually used because the child thread was unreachable",
        "Fallback route: not used, because the child thread was unreachable",
        "Fallback route: no route was used because the child thread was unreachable",
        "Fallback route: no route actually used because the child thread was unreachable",
        "Fallback route: no fallback route was actually used because the child thread was unreachable",
        "Fallback route: no fallback route actually used because the child thread was unreachable",
        "Fallback route: no fallback path actually used because the child thread was unreachable",
        "Fallback route: wasn't used because the child thread was unreachable",
        "Fallback route: weren't used because the child thread was unreachable",
        "Fallback route: no fallback was used because the child thread was unreachable",
        "Fallback route: no alternate route was used because the child thread was unreachable",
        "Fallback route: no alternate path actually used because the child thread was unreachable",
        "Fallback route: did not use the child thread because it was unreachable",
        "Fallback route: did not route the handoff because the child thread was unreachable",
        "Fallback route: did not route, because the child thread was unreachable",
        "Fallback route: did not route through the child thread because it was unreachable",
        "Fallback route: did not route through, because the child thread was unreachable",
        "Fallback route: did not route to the child thread because it was unreachable",
        "Fallback route: did not route via the child thread because it was unreachable",
        "Fallback route: could not route the handoff because the child thread was unreachable",
        "Fallback route: could not route to the child thread because it was unreachable",
        "Fallback route: cannot route to the child thread because it was unreachable",
        "Fallback route: failed to route to the child thread because it was unreachable",
        "Fallback route: unable to route to the child thread because it was unreachable",
        "Fallback route: not sent to the child owner because the handler failed",
        "Fallback route: not posted in the child thread because the handler failed",
        "Fallback route: not delivered in the child thread because the handler failed",
        "Fallback route: used; Tracking issue: #205",
        "Fallback route: ; Tracking issue: #205",
        "Fallback route: parent sent the handoff to the parent thread and then checked in the child thread",
        "Fallback route: parent sent the handoff to the parent thread and then spoke to the reviewer",
        "Fallback route: parent posted the handoffish to the child thread",
        "Fallback route: parent posted the handoff with reviewer feedback to the parent thread",
        "Fallback route: nonparent sent the handoff to the child thread",
        "Fallback route: grandparent sent the handoff to the child thread",
        "Fallback route: parent routed the feedback for reviewer notes in the parent thread",
        "Fallback route: no handoff was sent to the child owner because the handler failed",
        "Fallback route: no message was posted in the child thread because the handler failed",
        "Fallback route: was not actually successfully sent to the child owner because the handler failed",
        "Fallback route: handler failed",
        "Fallback route: handler failed, parent unavailable",
        "Fallback route: handler did not respond",
        "Fallback route: handler did not respond, parent unavailable",
        "Fallback route: handler didn't respond",
        "Fallback route: handler failure",
        "Fallback route: no handoff reached the child owner because the handler failed",
        "Fallback route: no message reached the child thread because the handler failed",
        "Fallback route: no handoff to the child thread",
        "Fallback route: no message to the child thread",
        "Fallback route: route not via the child thread",
        "Fallback route: path not in the child thread",
        "Fallback route: parent sent",
        "Fallback route: parent posted",
        "Fallback route: parent sent an unrelated note",
        "Fallback route: parent posted an unrelated note",
        "Fallback route: parent sent the handoff not to the child thread",
        "Fallback route: parent sent the handoff to someone other than the child thread",
        "Fallback route: parent routed the feedback to someone other than the reviewer",
        "Fallback route: parent delivered the message not to the child owner",
        "Fallback route: handler failed, parent sent no handoff to the child thread",
        "Fallback route: handler did not respond, parent posted no message in the child thread",
        "Fallback route: unused because the child thread was unreachable",
    ] {
        let output = run_ownership_validator(&base_evidence(route, "Tracking issue: #205"))?;

        assert!(
            !output.status.success(),
            "validator should reject longer negated fallback route values: {route}"
        );
    }
    Ok(())
}

#[test]
fn validator_allows_concrete_route_after_handler_failure_negation()
-> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: handler did not respond, parent sent the handoff to the child thread",
        "Fallback route: handler did not respond, parent posted the handoff to the child thread",
        "Fallback route: handler did not respond and parent sent the handoff to the child thread",
        "Fallback route: handler did not respond; parent sent the handoff to the child thread",
        "Fallback route: handler did not respond. Parent sent the handoff to the child thread",
        "Fallback route: handler did not respond to codex_app.read_thread, parent sent the handoff to the child thread",
        "Fallback route: handler did not respond via codex_app.read_thread, parent posted the handoff to the child thread",
        "Fallback route: parent posted the handoff in the child thread because read_thread was not available",
        "Fallback route: handler failed and parent sent the handoff to the child thread",
        "Fallback route: handler failed and parent posted the handoff to the child thread",
        "Fallback route: parent posted the handoff in the child thread",
        "Fallback route: parent delivered the message to the child owner",
        "Fallback route: parent routed the feedback to the reviewer",
    ] {
        let output = run_ownership_validator(&base_evidence(route, "Tracking issue: #205"))?;

        assert!(
            output.status.success(),
            "validator should allow concrete fallback route after unrelated handler negation: {route}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_metadata_before_defect_line() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: parent posted the handoff in the child thread
Tracking issue: #205
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include handoff metadata that precedes the defect line\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_metadata_before_invocation_then_defect()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: parent posted the handoff in the child thread
Tracking issue: #205
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include handoff metadata that precedes the invocation line when the defect follows it\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_metadata_before_same_line_defect() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: parent posted the handoff in the child thread
Tracking issue: #205
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread after `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include preceding handoff metadata before same-line defect captures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_metadata_before_bulleted_defect_line() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: parent posted the handoff in the child thread
Tracking issue: #205
Dogfooding/tool-exposure defect:
- Recorded runtime missing-handler evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include preceding handoff metadata before bulleted defect captures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
