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

fn base_evidence(route: &str, issue: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{route}
{issue}
Maintainer reassignment: none
"#
    )
}

#[test]
fn validator_rejects_negated_follow_up_issue_claim() -> Result<(), Box<dyn std::error::Error>> {
    for issue in [
        "No follow-up issue #205",
        "No separate follow-up issue #205",
        "Not a follow-up issue #205",
        "Follow-up issue: no issue was created for #205",
        "Follow-up issue: no issue has been created for #205",
        "Follow-up issue: no issue filed for #205",
        "Follow-up issue: no issue was filed for #205",
        "Follow-up issue: no issue has been filed for #205",
        "Follow-up issue: not filed for #205",
        "Follow-up issue: issue wasn't filed for #205",
        "Follow-up issue: issue has not been filed for #205",
    ] {
        let output = run_ownership_validator(&base_evidence(
            "Fallback route: no fallback route was available",
            issue,
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject negated follow-up issue claims: {issue}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_long_negated_fallback_value() -> Result<(), Box<dyn std::error::Error>> {
    for route in [
        "Fallback route: not used because the child thread was unreachable",
        "Fallback route: was not used because the child thread was unreachable",
        "Fallback route: not actually used because the child thread was unreachable",
        "Fallback route: not used, because the child thread was unreachable",
        "Fallback route: no route was used because the child thread was unreachable",
        "Fallback route: no route actually used because the child thread was unreachable",
        "Fallback route: no fallback route was actually used because the child thread was unreachable",
        "Fallback route: no fallback route actually used because the child thread was unreachable",
        "Fallback route: no fallback path actually used because the child thread was unreachable",
        "Fallback route: wasn't used because the child thread was unreachable",
        "Fallback route: weren't used because the child thread was unreachable",
        "Fallback route: no fallback was used because the child thread was unreachable",
        "Fallback route: no alternate route was used because the child thread was unreachable",
        "Fallback route: no alternate path actually used because the child thread was unreachable",
        "Fallback route: did not use the child thread because it was unreachable",
        "Fallback route: did not route through the child thread because it was unreachable",
        "Fallback route: did not route through, because the child thread was unreachable",
        "Fallback route: did not route to the child thread because it was unreachable",
        "Fallback route: did not route via the child thread because it was unreachable",
        "Fallback route: could not route to the child thread because it was unreachable",
        "Fallback route: cannot route to the child thread because it was unreachable",
        "Fallback route: unused because the child thread was unreachable",
        "Fallback route: unused, because the child thread was unreachable",
    ] {
        let output = run_ownership_validator(&base_evidence(route, "Tracking issue: #205"))?;

        assert!(
            !output.status.success(),
            "validator should reject longer negated fallback route values: {route}"
        );
    }
    Ok(())
}

#[test]
fn validator_allows_metadata_before_defect_line() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: parent posted the handoff in the child thread
Tracking issue: #205
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include handoff metadata that precedes the defect line\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_metadata_before_same_line_defect() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: parent posted the handoff in the child thread
Tracking issue: #205
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread after `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include preceding handoff metadata before same-line defect captures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_metadata_before_bulleted_defect_line() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: parent posted the handoff in the child thread
Tracking issue: #205
Dogfooding/tool-exposure defect:
- Recorded runtime missing-handler evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should include preceding handoff metadata before bulleted defect captures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
