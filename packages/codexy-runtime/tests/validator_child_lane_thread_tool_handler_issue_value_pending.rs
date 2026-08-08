use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

fn evidence_for(issue_evidence: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; {issue_evidence}.
Maintainer reassignment: none
"#
    )
}

#[test]
fn validator_rejects_absent_or_pending_tracking_issue_values()
-> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "tracking issue not opened yet #205",
        "tracking issue: not opened yet #205",
        "tracking issue: issue was not opened for #205",
        "tracking issue to be opened as #205",
        "follow-up issue should be filed as #205",
        "tracking issue needs to be filed as #205",
        "tracking issue planned as #205",
        "- tracking issue not opened yet #205",
        "1. tracking issue to be opened as #205",
    ] {
        let output = run_ownership_validator(&evidence_for(issue))?;
        assert!(
            !output.status.success(),
            "validator should reject pending tracking issue evidence `{issue}`"
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_concrete_tracking_issue_values_with_status_context()
-> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "tracking issue: #205 is not yet closed",
        "tracking issue: #205 covers handler not available",
        "tracking issue: #205 covers child thread not created",
        "- tracking issue: #205",
        "1. tracking issue: missing-handler exposure #205",
        "tracking issue: missing handler exposure #205",
    ] {
        let output = run_ownership_validator(&evidence_for(issue))?;
        assert!(
            output.status.success(),
            "validator should accept concrete tracking issue evidence `{issue}`"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_lifecycle_negated_issue_url_values() -> Result<(), Box<dyn std::error::Error>>
{
    for issue in [
        "tracking issue: https://github.com/eunsoogi/codexy/issues/205 not filed yet",
        "tracking issue: https://github.com/eunsoogi/codexy/issues/205 was not filed",
        "tracking issue: https://github.com/eunsoogi/codexy/issues/205 has not been created",
    ] {
        let output = run_ownership_validator(&evidence_for(issue))?;
        assert!(
            !output.status.success(),
            "validator should reject lifecycle-negated issue URL evidence `{issue}`"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_pending_issue_values_after_reference() -> Result<(), Box<dyn std::error::Error>>
{
    for issue in [
        "tracking issue: #205 will be created",
        "tracking issue: #205 (will be created)",
        "tracking issue: #205 will not be created",
        "tracking issue: #205 won't be created",
        "tracking issue: #205 is not yet created",
        "tracking issue: #205 (is not yet created)",
        "tracking issue: #205 will be filed",
        "tracking issue: #205 will not be filed",
        "tracking issue: #205 won't be filed",
        "tracking issue: #205 is not yet filed",
        "tracking issue: #205 to be created",
        "tracking issue: #205 should be created",
        "tracking issue: #205 needs to be created",
        "tracking issue: #205 to be filed",
        "tracking issue: #205 should be filed",
        "tracking issue: #205 needs to be filed",
        "tracking issue: #205 not opened yet",
        "tracking issue: #205 will be opened",
        "tracking issue: #205 to be opened",
        "tracking issue: #205 should be opened",
        "tracking issue: #205 needs to be opened",
    ] {
        let output = run_ownership_validator(&evidence_for(issue))?;
        assert!(
            !output.status.success(),
            "validator should reject pending tracking issue evidence after a reference `{issue}`"
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_concrete_issue_url_with_defect_context_negation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&evidence_for(
        "tracking issue: https://github.com/eunsoogi/codexy/issues/205 covers child thread not created",
    ))?;

    assert!(
        output.status.success(),
        "validator should accept concrete issue URLs when negation describes the defect, not issue lifecycle\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
