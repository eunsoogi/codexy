use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_placeholder_handler_missing_for_exposed_thread_tools()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Visible tool surface: list_threads, read_thread, send_message_to_thread, list_projects, and create_thread are exposed.
Invocation evidence: list_threads/read_thread/send_message_to_thread/list_projects/create_thread all return `No handler registered for tool: ...`.
Fallback: reported ordinary unavailable thread tooling and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject placeholder handler-missing evidence when exposed thread setup tools are named"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the placeholder missing-handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_split_line_placeholder_handler_missing_for_exposed_thread_tools()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Visible tool surface: list_threads, read_thread, send_message_to_thread, list_projects, and create_thread are exposed.
Invocation evidence: all return `No handler registered for tool: ...`.
Fallback: reported ordinary unavailable thread tooling and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should associate split-line placeholder handler evidence with the exposed thread tool list"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the split-line placeholder missing-handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_visible_thread_surface_handler_missing()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Visible tool surface: read_thread.
Invocation evidence: all return `No handler registered for tool: ...`.
Fallback: reported ordinary unavailable thread tooling and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should treat visible thread tool surfaces as discovered for missing-handler checks"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the visible-surface missing-handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_bulleted_visible_thread_surface_handler_missing()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Visible tool surface:
- read_thread
Invocation evidence: all return `No handler registered for tool: ...`.
Fallback: reported ordinary unavailable thread tooling and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should treat bulleted visible thread tool surfaces as discovered for missing-handler checks"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the bulleted visible-surface missing-handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_blank_line_separated_visible_surface_handler_missing()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Visible tool surface:
- read_thread

Invocation evidence: all return `No handler registered for tool: ...`.
Fallback: reported ordinary unavailable thread tooling and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should associate blank-line-separated placeholder handler evidence with the exposed thread tool list"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the blank-line-separated placeholder evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_broader_bulleted_visible_thread_surface_handler_missing()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Visible tool surface:
- apply_patch
- read_thread
Invocation evidence: all return `No handler registered for tool: ...`.
Fallback: reported ordinary unavailable thread tooling and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should preserve visible-surface discovery across non-thread bullets"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the broad-list missing-handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_placeholder_handler_missing_after_invocation_header()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Visible tool surface:
- read_thread
Invocation evidence:
all return `No handler registered for tool: ...`.
Fallback: reported ordinary unavailable thread tooling and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should associate placeholder handler evidence across invocation headers"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the header-separated placeholder evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
