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

fn lane_prefix_fixture(defect_and_metadata: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
{defect_and_metadata}
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_header_capture_borrowing_another_lane_handoff()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&lane_prefix_fixture(
        r#"Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: no fallback route was available
Tracking issue: #205
Lane B dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
"#,
    ))?;

    assert!(
        !output.status.success(),
        "validator must not let a Lane B defect capture borrow Lane A handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_bulleted_capture_borrowing_another_lane_handoff()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&lane_prefix_fixture(
        r#"Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- Lane A: recorded runtime missing-handler evidence for codex_app.read_thread.
- Lane B fallback route: no fallback route was available
- Lane B tracking issue: #205
"#,
    ))?;

    assert!(
        !output.status.success(),
        "validator must retain a bulleted Lane A capture label when scoping handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_preceding_metadata_from_another_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(&lane_prefix_fixture(
        r#"Lane B fallback route: no fallback route was available
Lane B tracking issue: #205
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
"#,
    ))?;

    assert!(
        !output.status.success(),
        "validator must strip a leading Lane B prefix before classifying preceding handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
