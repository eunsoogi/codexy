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
fn validator_allows_negated_other_lane_wording_before_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent captured tool exposure mismatch for the same lane, not another lane.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve same-lane metadata that negates another-lane wording\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_negated_other_lane_alias_before_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent captured tool exposure mismatch for the same lane, not the other lane.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve same-lane metadata that negates other-lane wording\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_preceding_metadata_that_negates_later_lane_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Fallback route: parent posted the handoff in the child thread for Lane A, not another lane.
Tracking issue: #246
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow preceding metadata that negates later-lane scope\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_same_lane_metadata_that_negates_explicit_lane_labels()
-> Result<(), Box<dyn std::error::Error>> {
    for phrase in [
        "not Lane B",
        "not for Lane B",
        "not in Lane B",
        "not assigned to Lane B",
        "not targeting Lane B",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent captured the missing-handler handoff for this lane, {phrase}.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should preserve same-lane metadata that says {phrase}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_same_lane_metadata_with_positive_other_lane_labels()
-> Result<(), Box<dyn std::error::Error>> {
    for phrase in [
        "for Lane B",
        "in Lane B",
        "assigned to Lane B",
        "targeting Lane B",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent captured the missing-handler handoff {phrase}.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject same-lane evidence that borrows metadata {phrase}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
