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
fn validator_rejects_handler_defect_using_fields_from_later_unrelated_defect()
-> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Dogfooding defect: unrelated plugin issue recorded no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread. Dogfooding defect: unrelated plugin issue recorded no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; Dogfooding defect: unrelated plugin issue recorded no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
- unrelated plugin issue recorded no fallback route was available; separate dogfood issue: #205
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            !output.status.success(),
            "validator should bind route/tracking fields to the handler defect, not a later unrelated defect"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_parent_route_with_later_child_thread_check()
-> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: parent sent the handoff to the parent thread, then checked in the child thread",
        "Fallback route: parent sent the handoff to the parent thread, then checked in the child thread and then confirmed status",
        "Fallback route: parent sent the handoff to the parent thread then checked in the child thread",
        "Fallback route: parent sent the handoff to the parent thread and later checked in the child thread",
        "Fallback route: parent sent the handoff to the parent thread before checking in the child thread",
        "Fallback route: parent sent the handoff to the parent thread after checking in the child thread",
        "Fallback route: parent sent the handoff to the parent thread and subsequently checked in the child thread",
    ] {
        let output = run_ownership_validator(&evidence(route))?;

        assert!(
            !output.status.success(),
            "validator should not use follow-up child-thread checks as fallback route destinations: {route}"
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_concrete_routes_with_later_status_context()
-> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: parent sent the handoff to the child thread and later checked in the child thread",
        "Fallback route: handler failed before parent sent the handoff to the child thread",
        "Fallback route: parent sent the handoff later to the child thread",
    ] {
        let output = run_ownership_validator(&evidence(route))?;

        assert!(
            output.status.success(),
            "validator should keep concrete fallback route wording valid: {route}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
