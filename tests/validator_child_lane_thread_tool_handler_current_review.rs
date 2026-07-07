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
fn validator_allows_handler_capture_under_generic_metadata_key()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Evidence: dogfooding/tool-exposure defect recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept handler defect captures recorded under a generic metadata key\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_preceding_non_bulleted_handler_defect_capture()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept a non-bulleted defect capture immediately before the missing-handler line\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_inline_defect_capture_after_unrelated_no_defect_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
LSP evidence: no dogfooding defect; rust-analyzer was unavailable on PATH.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread after `No handler registered for tool: read_thread`; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not expand an inline defect capture into unrelated prior no-defect metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_defect_capture_after_invocation_with_prior_no_defect_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
LSP evidence: no dogfooding defect; rust-analyzer was unavailable on PATH.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not expand an invocation capture into unrelated prior no-defect metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_bulleted_invocation_capture_after_prior_no_defect_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
LSP evidence: no dogfooding defect; rust-analyzer was unavailable on PATH.
- Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; separate dogfood issue: #205.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not expand a bulleted invocation capture into unrelated prior no-defect metadata\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_placeholder_capture_without_handler_defect_on_capture_line()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: ...`.
Dogfooding defect: recorded child ownership routing defect and tracked follow-up.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject placeholder captures unless the defect capture itself mentions handler-missing evidence"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_negated_placeholder_fallback_capture() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: ...`.
Dogfooding/tool-exposure defect: missing-handler evidence was not captured as an ordinary unavailable-tool fallback.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject placeholder fallback negation without affirmative defect capture"
    );
    Ok(())
}

#[test]
fn validator_rejects_combined_defect_label_capture_negation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: runtime missing-handler evidence for codex_app.read_thread, not captured as a dogfooding/tool-exposure defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject capture wording negated against the combined dogfooding/tool-exposure label"
    );
    Ok(())
}

#[test]
fn validator_rejects_later_clause_handler_failure_after_prior_negation()
-> Result<(), Box<dyn std::error::Error>> {
    for invocation_line in [
        "Invocation evidence: read_thread did not fail with `No handler registered for tool: read_thread`, but send_message_to_thread failed with `No handler registered for tool: send_message_to_thread`.",
        "Invocation evidence: read_thread did not fail with `No handler registered for tool: read_thread` but send_message_to_thread failed with `No handler registered for tool: send_message_to_thread`.",
    ] {
        let output = run_ownership_validator(&format!(
            "Owner decision: parent-owned for thread/worktree tool discovery only; child routing required\nTool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.\n{invocation_line}\nFallback: treated send_message_to_thread as an ordinary unavailable-tool fallback.\nMaintainer reassignment: none\n"
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject a later-clause handler failure that is not captured as a defect\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_handler_failure_after_unrelated_did_not_produce()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.send_message_to_thread as an available thread tool.
Invocation evidence: send_message_to_thread did not produce a response and failed with `No handler registered for tool: send_message_to_thread`.
Fallback: treated send_message_to_thread as an ordinary unavailable-tool fallback.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject a real handler failure when did-not-produce modifies a different object\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_tool_only_defect_capture_without_handler_marker()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: tracked codex_app.read_thread in review-response evidence.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should require the defect capture itself to mention handler registration or missing-handler evidence"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
