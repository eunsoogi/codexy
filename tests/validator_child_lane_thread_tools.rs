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

#[test]
fn validator_allows_absence_blocker_when_thread_events_are_negated()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: no codex_app namespace and no true thread tools exposed.
Unavailable-tool evidence: no thread/start or turn/start events were observed.
Blocker: thread tools unavailable because tool_search did not expose real thread tools.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow legitimate thread-tool blockers when thread events are explicitly absent\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_false_blocker_when_thread_events_are_split_across_lines()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: no codex_app namespace and no true thread tools exposed.
Event evidence: app-server-observed thread/start from a fresh child lane.
Event evidence: app-server-observed turn/start from the same fresh child lane.
Blocker: thread tools unavailable because tool_search did not expose real thread tools.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject false blockers when affirmative thread/start and turn/start evidence appears on separate lines"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("thread-tool discovery evidence"),
        "stderr should name the false blocker, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_negated_forbidden_satisfied_by_claims() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Thread requirement: satisfied by codex_app thread/start; not satisfied by codex exec.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow evidence that explicitly negates forbidden satisfied-by fallback claims\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_forbidden_satisfied_by_with_unrelated_prefix_negation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Thread requirement: not reviewed, satisfied by codex exec.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject affirmative forbidden satisfied-by claims when unrelated earlier words are negated"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Codex CLI"),
        "stderr should name the forbidden fallback, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_mixed_forbidden_fallback_claims_per_surface()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Thread fallback: used codex exec as a substitute; did not use codex app-server fallback.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject an affirmative codex exec substitute even when a different forbidden surface is negated"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Codex CLI"),
        "stderr should name the forbidden fallback, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Thread fallback: used codex exec as a substitute and did not use codex app-server fallback.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject an affirmative codex exec substitute even when a same-clause different surface is negated"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Codex CLI"),
        "stderr should name the forbidden fallback, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
