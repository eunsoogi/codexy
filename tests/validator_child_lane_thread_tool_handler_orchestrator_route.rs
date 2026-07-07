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

fn evidence(route: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{route}
Tracking issue: #205
Maintainer reassignment: none
"#
    )
}

#[test]
fn validator_allows_orchestrator_authored_fallback_route() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(&evidence(
        "Fallback route: orchestrator posted the handoff in the child thread",
    ))?;
    assert!(
        output.status.success(),
        "validator should accept concrete orchestrator-authored fallback route evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_vague_orchestrator_fallback_route() -> Result<(), Box<dyn std::error::Error>> {
    let output =
        run_ownership_validator(&evidence("Fallback route: orchestrator posted the handoff"))?;
    assert!(
        !output.status.success(),
        "validator should reject orchestrator route evidence without a concrete destination"
    );
    Ok(())
}

#[test]
fn validator_rejects_non_orchestrator_fallback_route() -> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: non orchestrator posted the handoff in the child thread",
        "Fallback route: non / orchestrator posted the handoff in the child thread",
        "Fallback route: non / - orchestrator posted the handoff in the child thread",
        "Fallback route: non - / orchestrator posted the handoff in the child thread",
        "Fallback route: no / - orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not actual orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not an orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not an actual orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not a real orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not / the orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the actual orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the real orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not-the orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not-the actual orchestrator posted the handoff in the child thread",
        "Fallback route: non/orchestrator posted the handoff in the child thread",
        "Fallback route: non \u{2013} orchestrator posted the handoff in the child thread",
        "Fallback route: non-orchestrator posted the handoff in the child thread",
        "Fallback route: non\u{2013}orchestrator posted the handoff in the child thread",
        "Fallback route: non_orchestrator posted the handoff in the child thread",
    ] {
        let output = run_ownership_validator(&evidence(route))?;
        assert!(
            !output.status.success(),
            "validator should reject non-orchestrator route evidence: {route}"
        );
    }
    Ok(())
}
