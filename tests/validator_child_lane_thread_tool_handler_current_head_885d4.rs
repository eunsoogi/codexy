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
fn validator_rejects_markdown_wrapped_pending_issue_urls() -> Result<(), Box<dyn std::error::Error>>
{
    for issue in [
        "Tracking issue: [dogfood](https://github.com/eunsoogi/codexy/issues/205) not filed yet",
        "Tracking issue: [dogfood](https://github.com/eunsoogi/codexy/issues/205) not created",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; {issue}.
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject Markdown-wrapped issue URLs followed by pending lifecycle text: {issue}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_negated_fallback_path_no_route_claims()
-> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: it is false that no fallback path was available",
        "Fallback route: not true that no fallback path was available",
        "Fallback route: not the case that no fallback path was available",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{route}
Tracking issue: #205
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject fallback-path no-route negation: {route}"
        );
    }
    Ok(())
}

#[test]
fn validator_allows_issue_descriptions_with_not_provided_text()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; tracking issue: #205 covers the handler not provided by the Codex app runtime.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow concrete issue references whose description contains not provided\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_negated_tracked_by_issue_clauses() -> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "not tracked by issue #205",
        "not tracked in issue #205",
        "not tracked by a separate issue #205",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; {issue}.
Maintainer reassignment: none
"#,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject negated tracked-by issue evidence: {issue}"
        );
    }
    Ok(())
}
