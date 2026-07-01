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

fn vague_fallback_evidence(route_evidence: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; {route_evidence}; separate dogfood issue: #205.
Maintainer reassignment: none
"#
    )
}

fn tracking_issue_evidence(issue_evidence: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; {issue_evidence}.
Maintainer reassignment: none
"#
    )
}

#[test]
fn validator_rejects_bare_fallback_route_used() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&vague_fallback_evidence("fallback route used"))?;

    assert!(
        !output.status.success(),
        "validator should reject fallback evidence that does not name the route used"
    );
    Ok(())
}

#[test]
fn validator_rejects_bare_fallback_routed() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&vague_fallback_evidence("fallback routed"))?;

    assert!(
        !output.status.success(),
        "validator should reject routed evidence that does not name the route"
    );
    Ok(())
}

#[test]
fn validator_rejects_weak_fallback_route_value() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&vague_fallback_evidence("fallback route: used"))?;

    assert!(
        !output.status.success(),
        "validator should reject weak fallback route values"
    );
    Ok(())
}

#[test]
fn validator_rejects_negated_no_route_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&vague_fallback_evidence(
        "no fallback route available evidence was not provided",
    ))?;

    assert!(
        !output.status.success(),
        "validator should reject negated explicit no-route evidence"
    );
    Ok(())
}

#[test]
fn validator_allows_tracking_issue_for_missing_handler_exposure()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&tracking_issue_evidence(
        "separate dogfood issue: #205 tracks the missing-handler exposure",
    ))?;

    assert!(
        output.status.success(),
        "validator should allow issue references that describe missing-handler exposure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_missing_tracking_issue_field_value() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(&tracking_issue_evidence("tracking issue: missing"))?;

    assert!(
        !output.status.success(),
        "validator should reject placeholder tracking issue field values"
    );
    Ok(())
}
