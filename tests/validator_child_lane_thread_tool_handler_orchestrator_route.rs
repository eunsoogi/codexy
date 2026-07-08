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
    for route in [
        "Fallback route: orchestrator posted the handoff in the child thread",
        "Fallback route: handler did not respond, orchestrator posted the handoff in the child thread",
        "Fallback route: handler did not verify. Orchestrator posted the handoff in the child thread",
        "Fallback route: handler did not verify, orchestrator posted the handoff in the child thread",
        "Fallback route: handler did not verify: parent posted the handoff in the child thread",
        "Fallback route: handler did not confirm readiness, orchestrator posted the handoff in the child thread",
        "Fallback route: handler did not actually verify, orchestrator posted the handoff in the child thread",
        "Fallback route: handler didn't verify, orchestrator posted the handoff in the child thread",
        "Fallback route: handler did not verify - parent posted the handoff in the child thread",
        "Fallback route: handler did not verify-parent posted the handoff in the child thread",
        "Fallback route: handler did not verify / parent posted the handoff in the child thread",
        "Fallback route: handler did not verify/parent posted the handoff in the child thread",
        "Fallback route: handler did not verify _ parent posted the handoff in the child thread",
        "Fallback route: handler did not verify_parent posted the handoff in the child thread",
        "Fallback route: handler did not verify—parent posted the handoff in the child thread",
        "Fallback route: handler ignored the prompt - parent posted the handoff in the child thread",
        "Fallback route: handler skipped the prompt - parent posted the handoff in the child thread",
        "Fallback route: we verified: parent posted the handoff in the child thread",
        "Fallback route: child owner available; parent posted the handoff in the child thread",
        "Fallback route: parent posted the handoff in the child thread, but ignored unrelated lint warnings",
        "Fallback route: parent posted the handoff in the child thread, but skipped unrelated optional cleanup",
    ] {
        let output = run_ownership_validator(&evidence(route))?;
        assert!(
            output.status.success(),
            "validator should accept concrete orchestrator-authored fallback route evidence: {route}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
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
        "Fallback route: no actual orchestrator posted the handoff in the child thread",
        "Fallback route: no authorized orchestrator posted the handoff in the child thread",
        "Fallback route: no single authorized orchestrator posted the handoff in the child thread",
        "Fallback route: no one from the orchestrator posted the handoff in the child thread",
        "Fallback route: nobody from the orchestrator posted the handoff in the child thread",
        "Fallback route: neither parent nor orchestrator posted the handoff in the child thread",
        "Fallback route: no member of the orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, no actual orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not any orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not any actual orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not actual orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not an orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not an actual orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not a real orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not / the orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the actual orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the current orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the primary orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the real orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not the right orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not-the orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not-the actual orchestrator posted the handoff in the child thread",
        "Fallback route: did not confirm orchestrator posted the handoff in the child thread",
        "Fallback route: did not verify that orchestrator posted the handoff in the child thread",
        "Fallback route: did not confirm that the actual orchestrator posted the handoff in the child thread",
        "Fallback route: did not verify whether orchestrator posted the handoff in the child thread",
        "Fallback route: did not verify if orchestrator posted the handoff in the child thread",
        "Fallback route: did not verify if any orchestrator posted the handoff in the child thread",
        "Fallback route: did not verify whether any actual orchestrator posted the handoff in the child thread",
        "Fallback route: did not verify if one authorized orchestrator posted the handoff in the child thread",
        "Fallback route: did not verify if a single authorized orchestrator posted the handoff in the child thread",
        "Fallback route: we did not verify: parent posted the handoff in the child thread",
        "Fallback route: we did not confirm; parent posted the handoff in the child thread",
        "Fallback route: we did not need a child owner; parent posted the handoff in the child thread",
        "Fallback route: no child owner; parent posted the handoff in the child thread",
        "Fallback route: did not confirm if orchestrator posted the handoff in the child thread",
        "Fallback route: did not actually verify orchestrator posted the handoff in the child thread",
        "Fallback route: did not fully verify that orchestrator posted the handoff in the child thread",
        "Fallback route: didn't verify that orchestrator posted the handoff in the child thread",
        "Fallback route: didn't verify if orchestrator posted the handoff in the child thread",
        "Fallback route: didn't actually verify that orchestrator posted the handoff in the child thread",
        "Fallback route: did not confirm the actual orchestrator posted the handoff in the child thread",
        "Fallback route: did not prove the real orchestrator posted the handoff in the child thread",
        "Fallback route: did not verify the right orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not actually orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not really orchestrator posted the handoff in the child thread",
        "Fallback route: handler failed, not truly orchestrator posted the handoff in the child thread",
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
