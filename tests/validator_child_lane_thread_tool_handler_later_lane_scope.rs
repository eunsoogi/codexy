use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_uncaptured_handler_before_later_same_lane_review_metadata_and_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B:
Fallback route: parent captured tool exposure mismatch for the same lane.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject same-lane review metadata plus defect capture from a later lane"
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_lowercase_multi_letter_lane_before_later_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane alpha:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane beta:
Fallback route: parent captured tool exposure mismatch for the same lane.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane beta.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject a later lowercase multi-letter lane capture for an earlier lane"
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_lowercase_multi_letter_lane_before_unqualified_later_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane alpha:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane beta:
Fallback route: parent captured tool exposure mismatch for the same lane.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject unqualified later capture under a lowercase multi-letter lane"
    );
    Ok(())
}

#[test]
fn validator_allows_lowercase_multi_letter_lane_with_generic_lane_setup_context()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane alpha:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent posted the handoff in the child thread in lane setup context.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane alpha.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not parse generic lane setup prose as a lowercase multi-letter lane label\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_lowercase_multi_letter_lane_with_this_lane_prose()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane alpha:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent posted the handoff in the child thread because this lane is child-owned.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane alpha.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not parse ordinary this-lane prose as a lowercase lane label\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_handler_before_later_same_lane_review_metadata_without_current_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B:
Fallback route: parent captured tool exposure mismatch for the same lane.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject later same-lane metadata when no current lane is established"
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_handler_before_inline_later_lane_same_lane_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Lane A invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Lane B fallback route: parent captured tool exposure mismatch for the same lane.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject inline later-lane metadata before an unqualified capture"
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_handler_before_later_unlabeled_lane_metadata_and_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: parent captured tool exposure mismatch for another lane.
Tracking issue: #999
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane B.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject later unlabeled lane metadata plus defect capture"
    );
    Ok(())
}

#[test]
fn validator_rejects_uncaptured_handler_before_other_lane_metadata_and_same_lane_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Fallback route: no fallback route was available for the other lane.
Tracking issue: #246
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject Lane A handler evidence that borrows handoff metadata scoped to the other lane\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
