use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
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

#[test]
fn validator_rejects_false_tracking_issue_answers() -> Result<(), Box<dyn std::error::Error>> {
    for issue in ["Tracking issue: #205? no", "tracked by issue #205: false"] {
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
            "validator should reject false tracking issue answer: {issue}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_wrapped_pending_issue_references() -> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "Tracking issue: [#205] not filed yet",
        "Tracking issue: (#205) will be created",
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
            "validator should reject wrapped pending issue reference: {issue}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_split_off_tracking_issue_negations() -> Result<(), Box<dyn std::error::Error>>
{
    for issue in [
        "tracking issue: #205; not a tracking issue for this defect",
        "tracking issue: #205. not a separate tracking issue for this defect",
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
            "validator should reject split-off tracking issue negation: {issue}"
        );
    }
    Ok(())
}
