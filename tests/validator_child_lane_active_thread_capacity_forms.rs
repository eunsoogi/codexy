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
fn validator_rejects_raw_create_thread_over_active_cap() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
create_thread(title="Codexy #269 active child cap")
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should treat raw create_thread transcript lines as child-thread operations"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity creation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_documented_codex_app_thread_over_active_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex app threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child Codex app thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should treat documented Codex app child-thread wording as a child-thread operation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity creation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_ignores_negated_thread_tool_policy_line() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Policy: MUST NOT call create_thread, fork_thread, or send_message_to_thread while actionable review exists.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat negated tool-name policy prose as an actual child-thread operation\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_new_thread_after_ledger_removal_frees_capacity()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Child thread thread-101 finished and was removed from the ledger.
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #270.
Thread creation: created child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should treat completed child ledger removal as freed capacity\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
