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
