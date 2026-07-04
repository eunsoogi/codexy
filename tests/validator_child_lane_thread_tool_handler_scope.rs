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
fn validator_allows_documented_capture_before_unrelated_no_defect_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
LSP evidence: no dogfooding defect; rust-analyzer was unavailable on PATH.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not let unrelated later no-defect evidence negate an already captured handler defect\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_documented_capture_before_unrelated_recorded_no_defect_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
LSP evidence: recorded no dogfooding defect; rust-analyzer was unavailable on PATH.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should end handler capture scope before unrelated metadata even when that metadata uses capture verbs\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_keeps_same_lane_header_metadata_inside_handler_capture_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Issue: #246
Branch: codexy/246-same-lane-header-scope
PR: #247
Head: abc1234
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should keep same-lane header metadata in handler capture scope\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_keeps_review_lane_header_metadata_inside_handler_capture_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane A:
Fallback route: parent captured tool exposure mismatch for the same lane.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should keep review lane header metadata in handler capture scope\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_keeps_lane_qualified_same_lane_capture_inside_handler_capture_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane A:
Fallback route: parent captured tool exposure mismatch for the same lane.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should keep lane-qualified same-lane capture in handler capture scope\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_ends_handler_capture_before_later_lane_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Lane B:
Dogfooding defect: none for an unrelated lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should end handler capture before later lane metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_handler_before_later_lane_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B:
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject later-lane defect capture for an earlier uncaptured handler failure"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing-handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_handler_before_later_lane_metadata_and_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B:
Issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject later-lane metadata plus defect capture for an earlier uncaptured handler failure"
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_handler_before_later_review_lane_metadata_and_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B:
Fallback route: parent captured tool exposure mismatch for another lane.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject later review-lane metadata plus defect capture for an earlier uncaptured handler failure"
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_handler_before_later_review_lane_metadata_and_unqualified_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B:
Fallback route: parent captured tool exposure mismatch for another lane.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject later review-lane metadata plus unqualified defect capture for an earlier uncaptured handler failure"
    );
    Ok(())
}
