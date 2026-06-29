use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
fn validator_rejects_handler_missing_for_discovered_thread_tool() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.set_thread_title as an available thread tool.
Invocation evidence: codex_app.set_thread_title failed with `No handler registered for tool: set_thread_title`.
Fallback: treated the failure as a transient unavailable-tool fallback and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject quiet fallback when a discovered thread tool has no registered handler"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_captured_handler_missing_dogfooding_defect() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding defect: handler-missing tool-exposure defect recorded with both the discovered codex_app.read_thread surface and the runtime handler failure.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow handler-missing evidence when it is captured as an actionable dogfooding defect\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_genuinely_unavailable_thread_tool_fallback() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: no codex_app namespace and no true thread tools exposed.
Unavailable-tool evidence: no thread/start or turn/start events were observed, and no thread tool invocation produced handler-missing evidence.
Fallback: reported unavailable thread tooling and stopped before child-owned implementation setup.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve normal fallback when thread tools are genuinely unavailable\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_negated_handler_missing_evidence() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: no codex_app namespace and no true thread tools exposed.
Unavailable-tool evidence: registered read_thread did not fail with `No handler registered for tool: read_thread`.
Fallback: reported unavailable thread tooling and stopped before child-owned implementation setup.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow evidence that explicitly negates handler-missing runtime failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_repeated_handler_missing_after_negated_occurrence() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.
Invocation evidence: codex_app.read_thread did not fail with `No handler registered for tool: read_thread`; codex_app.send_message_to_thread failed with `No handler registered for tool: send_message_to_thread`.
Fallback: treated the failure as an unavailable-tool fallback and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject a repeated handler-missing occurrence even when an earlier occurrence is negated"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_repeated_qualified_handler_missing_after_negated_occurrence() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread and codex_app.send_message_to_thread as available thread tools.
Invocation evidence: codex_app.read_thread did not fail with `No handler registered for tool: codex_app.read_thread`; codex_app.send_message_to_thread failed with `No handler registered for tool: codex_app.send_message_to_thread`.
Fallback: treated the failure as an unavailable-tool fallback and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject a repeated fully qualified handler-missing occurrence even when an earlier occurrence is negated"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_absent_handler_defect_capture() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding defect: none; missing handler was treated as an unavailable-tool fallback.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject handler-missing evidence when defect capture is explicitly absent"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_defect_scoped_not_captured_evidence() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: not captured runtime missing-handler evidence for codex_app.read_thread.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject defect-scoped not-captured evidence"
    );
    Ok(())
}

#[test]
fn validator_distinguishes_negated_capture_wording() -> TestResult {
    for (defect_line, should_pass) in [
        (
            "Dogfooding/tool-exposure defect: missing-handler evidence for codex_app.read_thread was not captured.",
            false,
        ),
        (
            "Dogfooding/tool-exposure defect: runtime missing-handler failures for codex_app.read_thread were not captured.",
            false,
        ),
        (
            "Dogfooding/tool-exposure defect: runtime missing-handler failures for codex_app.read_thread were not captured as an ordinary unavailable-tool fallback.",
            false,
        ),
        (
            "Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; it was not captured as an ordinary unavailable-tool fallback.",
            true,
        ),
        (
            "Dogfooding/tool-exposure defect: missing-handler evidence for codex_app.read_thread was not captured as an ordinary unavailable-tool fallback.",
            false,
        ),
        (
            "Dogfooding/tool-exposure defect: runtime missing-handler evidence for codex_app.read_thread, not captured as an ordinary unavailable-tool fallback.",
            false,
        ),
    ] {
        let output = run_ownership_validator(&format!(
            "Owner decision: parent-owned for thread/worktree tool discovery only; child routing required\nTool search: discovered codex_app.read_thread as an available thread tool.\nInvocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.\n{defect_line}\nMaintainer reassignment: none\n"
        ))?;
        assert_eq!(
            output.status.success(),
            should_pass,
            "unexpected validator result for `{defect_line}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_list_projects_handler_missing_for_thread_setup() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.create_thread as an available thread tool.
Project preflight: codex_app.list_projects failed with `No handler registered for tool: list_projects`.
Fallback: treated the failure as an unavailable-tool fallback and continued without recording a dogfooding defect.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject missing list_projects handlers because create_thread setup depends on project discovery"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No handler registered"),
        "stderr should name the missing handler evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
