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

fn separate_metadata_evidence(fallback_field: &str, tracking_field: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{fallback_field}
{tracking_field}
Maintainer reassignment: none
"#
    )
}

fn preceding_metadata_evidence() -> String {
    r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: no fallback route was available
Tracking issue: #205
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#
    .to_owned()
}

fn preceding_metadata_without_defect_evidence() -> String {
    r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: no fallback route was available
Tracking issue: #205
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#
    .to_owned()
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
fn validator_rejects_negated_fallback_route_not_used() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&vague_fallback_evidence("fallback route: not used"))?;
    assert!(
        !output.status.success(),
        "validator should reject negated fallback route values"
    );
    Ok(())
}
#[test]
fn validator_rejects_negated_fallback_route_not_routed() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&vague_fallback_evidence("fallback route: not routed"))?;
    assert!(
        !output.status.success(),
        "validator should reject negated fallback route values"
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
#[test]
fn validator_allows_handoff_fields_on_separate_metadata_lines()
-> Result<(), Box<dyn std::error::Error>> {
    for tracking_field in
        "Tracking issue: #205\nTracked in issue: #205\nTracked by issue: #205".lines()
    {
        let output = run_ownership_validator(&separate_metadata_evidence(
            "Fallback route: no fallback route was available",
            tracking_field,
        ))?;
        assert!(
            output.status.success(),
            "validator should accept separate metadata line `{tracking_field}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
#[test]
fn validator_allows_fallback_route_used_metadata_line() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&separate_metadata_evidence(
        "Fallback route used: parent posted the handoff in the child thread",
        "Tracking issue: #205",
    ))?;
    assert!(
        output.status.success(),
        "validator should accept fallback route used metadata"
    );
    Ok(())
}
#[test]
fn validator_allows_bulleted_handoff_fields_after_multiline_defect()
-> Result<(), Box<dyn std::error::Error>> {
    let evidence = r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread
- Fallback route: no fallback route was available
- Tracking issue: #205
Maintainer reassignment: none
"#;
    let output = run_ownership_validator(evidence)?;
    assert!(
        output.status.success(),
        "validator should accept bulleted handoff metadata"
    );
    Ok(())
}

#[test]
fn validator_allows_handoff_fields_before_raw_handler_line()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&preceding_metadata_evidence())?;
    assert!(
        output.status.success(),
        "validator should include preceding fallback and tracking metadata in handler capture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_preceding_metadata_without_defect_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&preceding_metadata_without_defect_evidence())?;
    assert!(
        !output.status.success(),
        "validator should not treat handoff metadata alone as a handler defect capture"
    );
    Ok(())
}

#[test]
fn validator_allows_github_issue_url_tracking_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&tracking_issue_evidence(
        "tracking issue: https://github.com/eunsoogi/codexy/issues/205",
    ))?;
    assert!(
        output.status.success(),
        "validator should accept GitHub issue URLs as concrete tracking issue evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_malformed_github_issue_url_suffix() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&tracking_issue_evidence(
        "tracking issue: https://github.com/eunsoogi/codexy/issues/205abc",
    ))?;
    assert!(
        !output.status.success(),
        "validator should reject malformed GitHub issue URL suffixes"
    );
    Ok(())
}
