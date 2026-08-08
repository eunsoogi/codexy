use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
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
        "Fallback route: no fallback path was available? no",
        "Fallback route: no fallback path was available: false",
        "Fallback route: parent sent the handoff to the child thread, but it was not used",
        "Fallback route: parent sent the handoff to the child thread and it was not used",
        "Fallback route: parent sent the handoff to the child thread; however it was not used",
        "Fallback route: parent posted the handoff in the child thread, but it was ignored",
        "Fallback route: parent posted the handoff in the child thread, but it was skipped",
        "Fallback route: parent posted the handoff in the child thread, but that was ignored",
        "Fallback route: parent posted the handoff in the child thread, but this was skipped",
        "Fallback route: parent posted the handoff in the child thread, but it got ignored",
        "Fallback route: parent posted the handoff in the child thread, but it got skipped",
        "Fallback route: parent posted the handoff in the child thread, but it gets ignored",
        "Fallback route: parent posted the handoff in the child thread, but it gets skipped",
        "Fallback route: parent posted the handoff in the child thread, but it has been ignored",
        "Fallback route: parent posted the handoff in the child thread, but it has been skipped",
        "Fallback route: parent posted the handoff in the child thread, but it is being ignored",
        "Fallback route: parent posted the handoff in the child thread, but it was being skipped",
        "Fallback route: parent posted the handoff in the child thread, but the handoff was ignored",
        "Fallback route: parent posted the handoff in the child thread, but the handoff was skipped",
        "Fallback route: parent posted the handoff in the child thread, but the message was ignored",
        "Fallback route: parent posted the handoff in the child thread, but the message was skipped",
        "Fallback route: parent posted the handoff in the child thread, but the thread was ignored",
        "Fallback route: parent posted the handoff in the child thread, but the handoff has been ignored",
        "Fallback route: parent posted the handoff in the child thread, but the message got skipped",
        "Fallback route: parent posted the handoff in the child thread, but the handoff is being ignored",
        "Fallback route: parent sent the handoff to the child thread, but the route was not used",
        "Fallback route: parent sent the handoff to the child thread, but that route was not used",
        "Fallback route: parent sent the handoff to the child thread, but the fallback route was not used",
        "Fallback route: parent sent the handoff to the child thread, but the send failed",
        "Fallback route: parent sent the handoff to the child thread, but it failed",
        "Fallback route: parent sent the handoff to the child thread and then the send failed",
        "Fallback route: parent posted the message in the child thread, but delivery failed",
        "Fallback route: parent posted the message in the child thread; the delivery failed",
        "Fallback route: parent posted the message in the child thread; delivery failure",
        "Fallback route: orchestrator posted the handoff in the child thread, but delivery-failure",
        "Fallback route: parent posted the handoff in the child thread, but delivery-failure",
        "Fallback route: parent posted the handoff in the child thread; delivery-failure",
        "Fallback route: parent delivered the message to the child owner, but the handoff failed to send",
        "Fallback route: parent sent the handoff to the child thread. the send failed",
        "Fallback route: parent posted the handoff in the child thread, then failed",
        "Fallback route: parent sent the handoff to the child thread, and the route was not used",
        "Fallback route: parent sent the handoff to the child thread and then the route was not used",
        "Fallback route: parent sent the handoff to the child thread and then it was not used",
        "Fallback route: parent sent the handoff to the child thread and then that route was not used",
        "Fallback route: parent sent the handoff to the parent thread and then the route was ignored in the child thread",
        "Fallback route: parent sent the handoff to the parent thread and then the route was skipped in the child thread",
        "Fallback route: parent posted the message to the parent thread and then the fallback route was ignored in the child thread",
        "Fallback route: orchestrator delivered the feedback to the parent thread and then that path was skipped in the child thread",
        "Fallback route: parent sent the handoff to the child thread; however the route was not used",
        "Fallback route: parent sent the handoff to the child thread, although the route was not used",
        "Fallback route: parent sent the handoff to the child thread, yet the route was not used",
        "Fallback route: parent sent the handoff to the child thread; however route was not used",
        "Fallback route: parent sent the handoff to the child thread, but this path was not used",
        "Fallback route: parent posted the handoff in the child thread, but route-unused",
        "Fallback route: parent posted the handoff in the child thread, but route/unused",
        "Fallback route: parent posted the handoff in the child thread, but route_unused",
        "Fallback route: parent posted the handoff in the child thread, but route-ignored",
        "Fallback route: parent posted the handoff in the child thread, but route-skipped",
        "Fallback route: parent posted the handoff in the child thread, but fallback route ignored",
        "Fallback route: parent posted the handoff in the child thread, but fallback path skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route was ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path was skipped",
        "Fallback route: parent posted the handoff in the child thread, but that route was ignored",
        "Fallback route: parent posted the handoff in the child thread, but this path was skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has been ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path has been skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has not been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route hasn't been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route got ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path got skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route gets ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path gets skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback has gotten ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback has gotten skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route is being ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path was being skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback is being ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route did not get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route didn't get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path did not get used",
        "Fallback route: parent posted the handoff in the child thread, but that route didn't get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has never been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route was never actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route hasn't actually been used",
        "Fallback route: parent posted the handoff in the child thread, but that route has never been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback was ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback was skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback has been ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback has been skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback has not been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback hasn't been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback did not get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback didn't get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback never actually got used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback has never been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback was never actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has not actually been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path has not actually been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback has not actually been used",
        "Fallback route: parent posted the handoff in the child thread, but that route has not actually been used",
        "Fallback route: parent posted the handoff in the child thread, but this path had not actually been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route was actually ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path was actually skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has actually been ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path has actually been skipped",
        "Fallback route: parent posted the handoff in the child thread, but that route had actually been ignored",
        "Fallback route: parent posted the handoff in the child thread, but this path had actually been skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback was actually ignored",
        "Fallback route: parent posted the handoff in the child thread, but the fallback has actually been skipped",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has not ever been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route hasn't ever been used",
        "Fallback route: parent posted the handoff in the child thread, but that route hadn't ever been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path was not ever used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback wasn't ever used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route did not ever get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback didn't ever get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has not actually ever been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback path had not actually ever been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback did not actually ever get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route was not actually ever used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route hadn't ever actually been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback hasn't ever actually been used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback wasn't ever actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route didn't ever actually get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback didn't ever actually get used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route hasn't gotten used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback never gets used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has not been actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has never been actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route hasn't been actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has not been ever used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route has never been ever used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback did not get actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback route didn't get actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback never got actually used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback did not get ever used",
        "Fallback route: parent posted the handoff in the child thread, but the fallback never ever got used",
    ] {
        let output = run_ownership_validator(&evidence(route))?;
        assert!(
            !output.status.success(),
            "validator should reject whole-event route negation: {route}"
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_orchestrator_fallback_route_events() -> Result<(), Box<dyn std::error::Error>>
{
    for route in [
        "Fallback route: orchestrator posted the handoff in the child thread",
        "Fallback route: orchestrator sent the message to the child owner",
        "Fallback route: orchestrator delivered the feedback to the reviewer",
        "Fallback route: parent sent the handoff to the parent thread and then the route was ignored in the child thread. Parent sent the handoff to the child thread",
        "Fallback route: parent sent the handoff to the parent thread and then the route was ignored in the child thread and then parent sent the handoff to the child thread",
        "Fallback route: parent sent the handoff to the parent thread and then the route was ignored in the child thread; parent sent the handoff to the child thread",
    ] {
        let output = run_ownership_validator(&evidence(route))?;
        assert!(
            output.status.success(),
            "validator should accept orchestrator-authored fallback route evidence: {route}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
