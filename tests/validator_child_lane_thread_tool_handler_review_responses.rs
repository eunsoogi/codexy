use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_uncaptured_handler_missing_in_later_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Lane A tool search: discovered codex_app.read_thread as an available thread tool.
Lane A invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding defect: handler-missing tool-exposure defect recorded with both the discovered codex_app.read_thread surface and the runtime handler failure; no fallback route was available; separate dogfood issue: #205.
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
Dogfooding defect: handler-missing tool-exposure defect recorded with both the discovered codex_app.read_thread surface and the runtime handler failure; no fallback route was available; separate dogfood issue: #205.
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
fn validator_rejects_documented_missing_handler_defect_without_handoff_fields()
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
        !output.status.success(),
        "validator should reject minimal missing-handler defect evidence without fallback/no-route and tracking issue fields"
    );
    Ok(())
}

#[test]
fn validator_allows_inline_defect_capture_before_handler_marker()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread with `No handler registered for tool: read_thread`; no fallback route was available; separate dogfood issue: #205.
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
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread, not captured as an ordinary unavailable-tool fallback; no fallback route was available; separate dogfood issue: #205.
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
fn validator_allows_capture_that_negates_unavailable_fallback_recording()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread, not recorded as an ordinary unavailable-tool fallback and without recording it as a normal fallback; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat fallback-recording negation as absent defect capture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_fallback_recorded_without_tool_exposure_defect()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback: recorded ordinary unavailable-tool fallback without recording tool-exposure defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject evidence that records fallback instead of the tool-exposure defect"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
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
- Recorded runtime missing-handler evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`; no fallback route was available; separate dogfood issue: #205.
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

#[test]
fn validator_rejects_prior_bullet_list_as_handler_defect_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect:
- Recorded discovered codex_app.read_thread as a callable thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback: treated the failure as unavailable thread tooling and stopped before child-owned implementation setup.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a prior bullet list satisfy a later uncaptured handler failure"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
