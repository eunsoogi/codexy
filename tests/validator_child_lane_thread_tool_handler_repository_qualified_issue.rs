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

fn tracking_issue_evidence(issue_evidence: &str) -> String {
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
fn validator_allows_repository_qualified_tracking_issue_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&tracking_issue_evidence(
        "tracking issue: eunsoogi/codexy#205",
    ))?;
    assert!(
        output.status.success(),
        "validator should accept repository-qualified issue references as concrete tracking issue evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_repository_qualified_tracking_issue_after_field_label()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&tracking_issue_evidence(
        "Tracking issue:eunsoogi/codexy#205",
    ))?;
    assert!(
        output.status.success(),
        "validator should accept repository-qualified issue references after a field label\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_markdown_link_github_issue_url_tracking_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&tracking_issue_evidence(
        "tracking issue: [dogfood issue](https://github.com/eunsoogi/codexy/issues/205)",
    ))?;
    assert!(
        output.status.success(),
        "validator should accept Markdown link GitHub issue URLs as concrete tracking issue evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_slash_delimited_bare_tracking_issue_references()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(&tracking_issue_evidence("tracking issue: #205/#206"))?;
    assert!(
        output.status.success(),
        "validator should accept slash-delimited bare issue references\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_punctuation_delimited_bare_tracking_issue_references()
-> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "tracking issue: (#205)",
        "Tracking issue:#205",
        "tracking issue: #205, covers the handler exposure",
        "tracking issue: #205 - missing handler",
    ] {
        let output = run_ownership_validator(&tracking_issue_evidence(issue))?;
        assert!(
            output.status.success(),
            "validator should accept punctuation-delimited bare issue reference `{issue}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_malformed_repository_qualified_issue_references()
-> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "tracking issue: https://github.com/issues/205",
        "tracking issue: https://github.com/eunsoogi/issues/205",
        "tracking issue: eunsoogi/codexy#",
        "tracking issue: eunsoogi/codexy#205abc",
        "tracking issue: eunsoogi/#205",
        "tracking issue: /codexy#205",
        "tracking issue: ../codexy#205",
        "tracking issue: eunsoogi/.#205",
        "tracking issue: abc#205",
        "tracking issue: codexy#205",
        "tracking issue: #205abc",
    ] {
        let output = run_ownership_validator(&tracking_issue_evidence(issue))?;
        assert!(
            !output.status.success(),
            "validator should reject malformed issue evidence `{issue}`"
        );
    }
    Ok(())
}
