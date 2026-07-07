use std::process::{Command, Output};

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?)
}

#[test]
fn validator_rejects_pre_action_route_not_used_value() -> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: was not used, parent sent the handoff to the child thread",
        "Fallback route: parent sent the handoff to the child thread, but it isn't used",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{route}
Tracking issue: #205
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject route values that say the fallback was not used: {route}"
        );
    }
    Ok(())
}

#[test]
fn validator_allows_preceding_no_route_handoff_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
No fallback route: no fallback route was available
Tracking issue: #205
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include preceding no-route metadata in backward handoff scans\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_lane_prefixed_preceding_handoff_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Status: parent only inspected the PR state for routing.

Lane B fallback route: parent sent the handoff to the child thread
Lane B tracking issue: #205
Lane B dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Lane B invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include same-lane prefixed preceding handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_current_tense_contracted_issue_negations()
-> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "tracking issue: #205 isn't created",
        "tracking issue: issue isn't filed for #205",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; {issue}.
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject current-tense contracted issue lifecycle negation: {issue}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_post_reference_issue_lifecycle_connectors()
-> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "tracking issue: #205, but was not filed",
        "tracking issue: #205 however not created",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; {issue}.
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject issue lifecycle negation after connector words: {issue}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_handoff_fields_from_comma_separated_unrelated_defect()
-> Result<(), Box<dyn std::error::Error>> {
    for separator in [",", " -"] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread{separator} Dogfooding defect: unrelated plugin issue recorded no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should not use comma/dash-separated unrelated defect handoff fields: {separator}"
        );
    }
    Ok(())
}

#[test]
fn validator_allows_bulleted_fallback_route_used_handoff_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- Recorded runtime missing-handler evidence for codex_app.read_thread.
- Fallback route used: parent posted the handoff in the child thread
- Tracking issue: #205
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept fallback route used as bulleted handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_did_not_use_fallback_route_followups() -> Result<(), Box<dyn std::error::Error>>
{
    for route in [
        "Fallback route: parent posted the handoff in the child thread, but didn't use it",
        "Fallback route: parent posted the handoff in the child thread, but did not use it",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{route}
Tracking issue: #205
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject did-not-use fallback route follow-up: {route}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_did_not_create_tracking_issue_text() -> Result<(), Box<dyn std::error::Error>>
{
    for issue in [
        "tracking issue: issue did not get created for #205",
        "tracking issue: issue didn't get created for #205",
        "tracking issue: issue did not create #205",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; {issue}.
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject did-not-create tracking issue text: {issue}"
        );
    }
    Ok(())
}
