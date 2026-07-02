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
fn validator_allows_one_defect_capture_for_same_line_handler_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.list_threads and codex_app.list_projects as available thread tools.
Invocation evidence: codex_app.list_threads failed with `No handler registered for tool: list_threads`; codex_app.list_projects failed with `No handler registered for tool: list_projects`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.list_threads and codex_app.list_projects; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept one following defect report for same-line handler failures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_same_line_handler_failure_missing_from_defect_report()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.list_threads and codex_app.list_projects as available thread tools.
Invocation evidence: codex_app.list_threads failed with `No handler registered for tool: list_threads`; codex_app.list_projects failed with `No handler registered for tool: list_projects`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.list_threads.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject grouped same-line handler evidence when the defect report omits one failed tool"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_one_defect_capture_for_adjacent_handler_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.list_threads and codex_app.list_projects as available thread tools.
Invocation evidence:
- codex_app.list_threads failed with `No handler registered for tool: list_threads`.
- codex_app.list_projects failed with `No handler registered for tool: list_projects`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.list_threads and codex_app.list_projects; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept one following defect report for adjacent handler failures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_lane_prefixed_handoff_fields_for_matching_lane_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Lane A tool search: discovered codex_app.read_thread as an available thread tool.
Lane A invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane A dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Lane A fallback route: no fallback route was available.
Lane A tracking issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept lane-prefixed fallback/tracking fields for the matching lane capture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_capture_that_negates_fallback_classification_and_reporting()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread, not classified as an ordinary unavailable-tool fallback and without reporting it as a fallback; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat fallback classification/reporting negation as absent defect capture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_defect_scoped_not_classified_capture() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: not classified runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject defect-scoped not-classified evidence"
    );
    Ok(())
}

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
Lane A tool search: discovered codex_app.read_thread as an available thread tool.
Lane A invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent posted the handoff in the child thread.
Tracking issue: #205.
Lane B dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
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
