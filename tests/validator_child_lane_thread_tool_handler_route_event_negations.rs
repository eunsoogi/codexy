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
fn validator_rejects_whole_event_route_negations() -> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: no, parent sent the handoff to the child thread",
        "Fallback route: not true, see parent thread",
        "Fallback route: false: parent sent the handoff to the child thread",
        "Fallback route: false positive parent sent the handoff to the child thread",
        "Fallback route: false-positive parent sent the handoff to the child thread",
        "Fallback route: it is false that parent sent the handoff to the child thread",
        "Fallback route: it is not true that parent sent the handoff to the child thread",
        "Fallback route: it is not the case that parent sent the handoff to the child thread",
        "Fallback route: parent sent the handoff to the child thread? no",
        "Fallback route: parent posted the handoff in the child thread? false",
        "Fallback route: parent sent the handoff to the child thread was not used",
        "Fallback route: never parent sent the handoff to the child thread",
        "Fallback route: unable, parent sent the handoff to the child thread",
        "Fallback route: parent sent the handoff to the child thread, but was not used",
        "Fallback route: parent sent the handoff to the child thread but was not used",
        "Fallback route: parent sent the handoff to the child thread, not used",
        "Fallback route: parent sent the handoff to the child thread was never used",
        "Fallback route: no - parent sent the handoff to the child thread",
        "Fallback route: no fallback route: parent sent the handoff to the child thread",
        "Fallback route: parent sent the handoff to the child thread, but it was not used",
        "Fallback route: parent sent the handoff to the child thread and it was not used",
        "Fallback route: parent sent the handoff to the child thread; however it was not used",
        "Fallback route: parent sent the handoff to the child thread, but the route was not used",
        "Fallback route: parent sent the handoff to the child thread, but that route was not used",
        "Fallback route: parent sent the handoff to the child thread, but the fallback route was not used",
        "Fallback route: parent sent the handoff to the child thread, but the send failed",
        "Fallback route: parent sent the handoff to the child thread, but it failed",
        "Fallback route: parent sent the handoff to the child thread and then the send failed",
        "Fallback route: parent posted the message in the child thread, but delivery failed",
        "Fallback route: parent posted the message in the child thread; the delivery failed",
        "Fallback route: parent posted the message in the child thread; delivery failure",
        "Fallback route: parent delivered the message to the child owner, but the handoff failed to send",
        "Fallback route: parent sent the handoff to the child thread. the send failed",
        "Fallback route: parent sent the handoff to the child thread, and the route was not used",
        "Fallback route: parent sent the handoff to the child thread and then the route was not used",
        "Fallback route: parent sent the handoff to the child thread and then it was not used",
        "Fallback route: parent sent the handoff to the child thread and then that route was not used",
        "Fallback route: parent sent the handoff to the child thread; however the route was not used",
        "Fallback route: parent sent the handoff to the child thread, although the route was not used",
        "Fallback route: parent sent the handoff to the child thread, yet the route was not used",
        "Fallback route: parent sent the handoff to the child thread; however route was not used",
        "Fallback route: parent sent the handoff to the child thread, but this path was not used",
    ] {
        let output = run_ownership_validator(&evidence(route))?;
        assert!(
            !output.status.success(),
            "validator should reject whole-event route negation: {route}"
        );
    }
    Ok(())
}
