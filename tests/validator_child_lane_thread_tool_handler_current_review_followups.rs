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
    let output = run_ownership_validator(&base_evidence(
        "Fallback route: not used because the child thread was unreachable",
        "Tracking issue: #205",
    ))?;

    assert!(
        !output.status.success(),
        "validator should reject longer negated fallback route values"
    );
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
