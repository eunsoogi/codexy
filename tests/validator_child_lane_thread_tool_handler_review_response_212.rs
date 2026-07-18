use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_allows_handoff_fields_after_exact_error_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Exact missing-handler error: `No handler registered for tool: read_thread`.
Fallback route: no fallback route was available
Tracking issue: #205
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should keep scanning handoff fields after exact-error metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_concrete_issue_with_no_extra_follow_up_needed()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: no fallback route was available
Tracking issue: #205; no follow-up issue needed beyond that
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept a concrete tracking issue when later text only says no extra follow-up issue is needed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_placeholder_with_exact_error_for_failed_tool_only()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: ...`.
Exact missing-handler error: `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: no fallback route was available
Tracking issue: #205
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not require exact errors for unrelated discovery-only tools\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_unused_fallback_route_followup() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: parent posted the handoff in the child thread, but unused
Tracking issue: #205
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject fallback route evidence that says the route was unused"
    );
    Ok(())
}

#[test]
fn validator_rejects_later_handler_defect_without_own_handoff_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: no fallback route was available
Tracking issue: #205
Invocation evidence: codex_app.send_message_to_thread failed with `No handler registered for tool: send_message_to_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.send_message_to_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a later handler defect borrow route/issue fields from the previous defect"
    );
    Ok(())
}

#[test]
fn validator_rejects_later_handler_defect_borrowing_prior_exact_error_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Exact missing-handler error: `No handler registered for tool: read_thread`.
Fallback route: no fallback route was available
Tracking issue: #205
Invocation evidence: codex_app.send_message_to_thread failed with `No handler registered for tool: send_message_to_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.send_message_to_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should keep exact-error and handoff metadata scoped to the prior defect"
    );
    Ok(())
}
