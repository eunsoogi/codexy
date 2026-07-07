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
fn validator_rejects_tool_only_defect_after_other_handler_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.
Invocation evidence: codex_app.send_message_to_thread failed with `No handler registered for tool: send_message_to_thread`.
Dogfooding defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Dogfooding defect: tracked codex_app.send_message_to_thread as a callable thread surface.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject capture scopes that do not couple the current tool with missing-handler semantics\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_handler_list_item_borrowing_unrelated_handoff_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread
- unrelated plugin issue recorded no fallback route was available; separate dogfood issue: #205
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject handler list items that borrow handoff fields from unrelated later bullets\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_handler_list_item_with_following_handoff_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread: `No handler registered for tool: read_thread`
Fallback route: no fallback route was available
Tracking issue: #205
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept handler list items with following unbulleted handoff metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_shared_handoff_metadata_after_multiple_handler_items()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.
Dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread: `No handler registered for tool: read_thread`
- recorded runtime missing-handler evidence for codex_app.send_message_to_thread: `No handler registered for tool: send_message_to_thread`
Fallback route: no fallback route was available
Tracking issue: #205
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should apply shared handoff metadata after a handler list to every handler item\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_hyphenated_fallback_route_fields() -> Result<(), Box<dyn std::error::Error>> {
    for field in "Fallback-route|Fallback-path".split('|') {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{field}: parent posted the handoff in the child thread.
Tracking issue: #205
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            output.status.success(),
            "validator should accept a hyphenated {field} field\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
