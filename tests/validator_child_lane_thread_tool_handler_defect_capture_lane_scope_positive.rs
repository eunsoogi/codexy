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
fn validator_allows_current_lane_tracking_issue_metadata_named_in_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: no fallback route was available for Lane A.
Tracking issue: #246 in Lane A review thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve tracking issue metadata explicitly scoped to the current lane\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_generic_in_lane_setup_context() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent posted the handoff in the child thread in lane setup context.
Tracking issue: #246.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not parse generic lane setup prose as a different lane label\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_multi_letter_lane_generic_setup_context()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane alpha:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent posted the handoff in the child thread in lane setup context.
Tracking issue: #246.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane alpha.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not parse generic lowercase lane setup prose as a multi-letter lane label\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
