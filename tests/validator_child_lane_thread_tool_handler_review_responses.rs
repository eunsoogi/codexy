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
fn validator_rejects_uncaptured_handler_missing_in_later_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Lane A tool search: discovered codex_app.read_thread as an available thread tool.
Lane A invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding defect: handler-missing tool-exposure defect recorded with both the discovered codex_app.read_thread surface and the runtime handler failure.
Lane B tool search: discovered codex_app.send_message_to_thread as an available thread tool.
Lane B invocation evidence: codex_app.send_message_to_thread failed with `No handler registered for tool: send_message_to_thread`.
Lane B fallback: treated the failure as an unavailable-tool fallback and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject an uncaptured later lane even when an earlier lane captured a handler defect"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_same_tool_handler_missing_in_later_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Lane A tool search: discovered codex_app.read_thread as an available thread tool.
Lane A invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding defect: handler-missing tool-exposure defect recorded with both the discovered codex_app.read_thread surface and the runtime handler failure.
Lane B tool search: discovered codex_app.read_thread as an available thread tool.
Lane B invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B fallback: treated the failure as an unavailable-tool fallback and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject an uncaptured same-tool later lane even when an earlier lane captured that tool's handler defect"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_non_thread_handler_error_in_later_sentence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool. Invocation evidence: tool_search failed with `No handler registered for tool: tool_search`.
Fallback: recorded tool_search separately and stopped before child-owned implementation setup.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not attribute a non-thread handler error to a thread tool mentioned in an earlier sentence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_documented_missing_handler_defect_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept the documented runtime missing-handler defect wording\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_inline_defect_capture_before_handler_marker()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept inline capture text before the handler marker\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_capture_that_negates_unavailable_fallback_reporting()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread, not reported as an ordinary unavailable-tool fallback.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat fallback-reporting negation as absent defect capture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_multiline_handler_defect_capture() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect:
- Recorded discovered codex_app.read_thread as a callable thread tool.
- Recorded runtime missing-handler evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept defect capture recorded across a scoped multiline block\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
