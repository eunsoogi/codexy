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
fn validator_rejects_waiting_child_thread_count_over_cap() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Waiting child Codex threads: 6
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should count waiting child threads toward the active/waiting cap"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("keep at most five active child Codex threads"),
        "stderr should name over-capacity waiting ledger, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_replacement_when_count_names_different_thread_for_issue()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5, including thread-other for issue #269.
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old was stopped as unusable for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject replacement neutrality when the count names a different thread for the same issue"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity replacement, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_explicit_total_after_thread_id_list() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: thread-101, thread-102, thread-103, thread-104, thread-105 (5 total)
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should parse explicit total instead of thread id digits\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_explicit_total_after_over_cap_thread_id_list()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: thread-001, thread-002, thread-003, thread-004, thread-005, thread-006 (6 total)
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject explicit over-cap total after thread id list"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("keep at most five active child Codex threads"),
        "stderr should name over-cap explicit total, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_total_label_after_thread_id_list() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: thread-101, thread-102, thread-103, thread-104, thread-105 (total: 5)
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should parse total label instead of thread id digits\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
