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
fn validator_allows_not_fallback_substitutes_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Forbidden route: codex exec, codex fork, and codex app-server commands are not fallback substitutes for true thread tools.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow evidence that says Codex CLI commands are not fallback substitutes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_satisfied_by_cli_even_with_not_fallback_substitute_wording()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Thread requirement: satisfied by codex exec, not a fallback substitute.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject affirmative codex exec satisfaction claims even when the line also says not a fallback substitute"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Codex CLI"),
        "stderr should name the forbidden fallback, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Thread requirement: satisfied by codex app-server, not a fallback substitute.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject affirmative codex app-server satisfaction claims even when the line also says not a fallback substitute"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Codex CLI"),
        "stderr should name the forbidden fallback, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn validator_allows_real_thread_surface_satisfied_by_with_negated_cli_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Thread requirement: satisfied by codex_app thread/start; did not use codex exec fallback.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow real thread surface satisfaction plus negated CLI fallback\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
