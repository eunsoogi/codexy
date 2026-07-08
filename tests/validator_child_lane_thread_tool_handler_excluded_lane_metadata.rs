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
fn validator_rejects_excluded_lane_metadata_that_names_a_later_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane ownership: child-owned for Lane B
Fallback route: parent captured tool exposure mismatch for the later lane.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not bridge a Lane A handler failure through ownership metadata for Lane B\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_excluded_lane_metadata_after_multiline_defect_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane A:
Fallback route: parent captured tool exposure mismatch for the same lane.
Tracking issue: #246
Dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread in Lane A; no fallback route was available; separate dogfood issue: #205.
Lane metadata: recorded no dogfooding defect for unrelated LSP evidence.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should end multiline handler captures before later excluded no-defect metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_handoff_metadata_that_names_a_later_lane()
-> Result<(), Box<dyn std::error::Error>> {
    for (field, qualifier) in [
        ("Fallback route", "Lane B"),
        ("Fallback path", "Lane B"),
        ("Fallback route", "another lane"),
        ("Fallback path", "later lane"),
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
{field}: parent captured tool exposure mismatch for {qualifier}.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should not bridge Lane A through {field} metadata for Lane B\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
