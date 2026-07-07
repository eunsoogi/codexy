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
fn validator_allows_same_line_old_owner_disposition_before_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4; Existing issue/PR owner check: existing owner thread thread-old found for issue #269; Old owner disposition: existing owner thread thread-old was stopped as unusable for issue #269; Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should honor same-line old-owner disposition before replacement\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_same_line_freed_capacity_before_refreshed_count_and_creation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: existing owner thread thread-269 found for issue #269.
Thread resume: continued child thread thread-269 for issue #269.
Child thread thread-old finished and was removed from the active ledger; Active child Codex threads: 4; Existing issue/PR owner check: no existing owner thread found for issue #270; Thread creation: created child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should reset projected capacity on same-line freed-capacity count\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_created_a_new_child_thread_over_cap() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created a new child Codex app thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should detect created-a-new child-thread operations"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity child thread creation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_trailing_same_line_removal_before_refreshed_count()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: existing owner thread thread-269 found for issue #269.
Thread resume: continued child thread thread-269 for issue #269.
Active child Codex threads: 5; Child thread thread-old finished and was removed from the active ledger.
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #270.
Thread creation: created child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve trailing same-line removal before refreshed count\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
