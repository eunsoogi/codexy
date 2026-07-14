use super::*;

#[test]
fn validator_rejects_uncaptured_earlier_lane_with_later_same_tool_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Lane A tool search: discovered codex_app.read_thread as an available thread tool.
Lane A invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane A fallback: treated the failure as unavailable thread tooling.
Lane B tool search: discovered codex_app.read_thread as an available thread tool.
Lane B invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a later same-tool defect report satisfy an earlier uncaptured lane"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_bulleted_earlier_lane_with_later_same_tool_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
- Lane A tool search: discovered codex_app.read_thread as an available thread tool.
- Lane A invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
- Lane A fallback: treated the failure as unavailable thread tooling.
- Lane B tool search: discovered codex_app.read_thread as an available thread tool.
- Lane B invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not group separate bulleted lane evidence into one shared defect report"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_metadata_bridged_earlier_lane_with_later_same_tool_capture()
-> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Lane A:
Fallback route: parent posted the handoff in the child thread.
Tracking issue: #205.
Lane B dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread after `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent posted the handoff in the child thread.
Tracking issue: #205.
Lane B dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Lane A tool search: discovered codex_app.read_thread as an available thread tool.
Lane A fallback route: parent posted the handoff in the child thread.
Lane A tracking issue: #205.
Lane B invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            !output.status.success(),
            "validator should not let handoff metadata bridge an earlier lane failure to a later same-tool defect"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
            "stderr should name the missing handler evidence, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_explicit_lane_defect_with_same_line_other_lane_handoff_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A; Lane B fallback route: no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let an explicit Lane A defect borrow same-line Lane B handoff fields"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_explicit_lane_defect_with_period_separated_other_lane_handoff_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A. Lane B fallback route: no fallback route was available. Separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let an explicit Lane A defect borrow period-separated Lane B handoff fields"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
