use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_placeholder_capture_without_exact_tool_error()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: ...`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject placeholder handler captures that do not include an exact tool error"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_placeholder_tool_covered_by_other_exact_error()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.
Invocation evidence:
- codex_app.read_thread failed with `No handler registered for tool: ...`.
- codex_app.send_message_to_thread failed with `No handler registered for tool: send_message_to_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.send_message_to_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject placeholder evidence for one tool even when another tool has an exact error"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_placeholder_with_following_exact_error_line()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: ...`.
Exact missing-handler error: `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept placeholder evidence followed by a separate exact handler error line\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
