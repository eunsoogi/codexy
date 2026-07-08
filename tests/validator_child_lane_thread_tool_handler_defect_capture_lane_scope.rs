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
fn validator_rejects_list_defect_borrowing_preceding_later_lane_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Fallback route: parent captured tool exposure mismatch for Lane B.
Tracking issue: #246
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- Lane A: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a Lane A list defect borrow preceding Lane B handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_list_defect_borrowing_trailing_later_lane_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- Lane A: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: parent posted the handoff in the child thread for Lane B.
Tracking issue: #246 in Lane B review thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a Lane A list defect borrow trailing Lane B handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_list_defect_borrowing_bulleted_later_lane_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- Lane A: recorded runtime missing-handler evidence for codex_app.read_thread.
- Fallback route: parent posted the handoff in the child thread for Lane B.
- Tracking issue: #246
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a Lane A list defect borrow bulleted Lane B handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_preceding_handoff_metadata_with_in_lane_later_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Fallback route: parent posted the handoff in the child thread in Lane B review thread.
Tracking issue: #246
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A; no fallback route was available.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not borrow preceding handoff metadata scoped in Lane B for a Lane A defect\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_preceding_tracking_issue_metadata_with_later_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Fallback route: no fallback route was available for Lane A.
Tracking issue: #246 in Lane B review thread.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not borrow a tracking issue scoped to Lane B for a Lane A defect\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_bulleted_preceding_metadata_for_another_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
- Fallback route: parent posted the handoff for another lane.
- Tracking issue: #246
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a Lane A defect borrow bulleted preceding metadata for another lane\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_multi_letter_lane_metadata_borrowing() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane alpha:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread.
- Fallback route: no fallback route was available for Lane beta.
- Tracking issue: #246 in Lane beta review thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let a Lane alpha defect borrow Lane beta handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_inline_multi_letter_lane_metadata_borrowing()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread in lane alpha.
- Fallback route: no fallback route was available for lane beta.
- Tracking issue: #246 in lane beta review thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let an inline lane alpha defect borrow lane beta handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
