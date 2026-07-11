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
fn validator_rejects_more_than_five_active_child_threads() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 6
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject orchestration evidence with six active child Codex threads"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("at most five active child Codex threads"),
        "stderr should name the active child thread cap, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_new_child_thread_when_five_are_already_active()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Packaged specialist subagents active: 8
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject new child thread creation when five child Codex threads are already active"
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
fn validator_allows_five_active_child_threads_without_new_thread_operation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Packaged specialist subagents active: 8
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow five active child Codex threads and not count packaged specialist subagents when no new child operation is attempted\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_new_child_thread_without_existing_owner_check()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject new child thread creation without existing issue/PR owner lookup"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing owner reuse evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_new_child_thread_without_active_count_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject new child thread creation without active child thread count evidence"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("active child Codex thread count"),
        "stderr should name missing active-count evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_replacement_child_thread_without_superseding_old_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-148 found for issue #269.
Thread creation: created replacement child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject replacement child thread creation when the old owner is not stopped, unusable, or explicitly superseded"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name the missing old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_replacement_child_thread_with_superseded_owner_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-148 found for issue #269.
Old owner disposition: thread-148 was stopped as unusable and explicitly superseded.
Thread creation: created replacement child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow replacement child thread creation after old owner evidence is inspected and superseded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_continuing_the_existing_owner_thread() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-148 found for issue #269.
Thread resume: continued child thread thread-148 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow continuing the same existing owner thread without old-owner supersession evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_unrelated_old_owner_disposition() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-148 found for issue #269.
Old owner disposition: thread-270 was stopped as unusable and explicitly superseded for issue #270.
Thread creation: created replacement child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject replacement when old-owner disposition is for an unrelated issue/thread"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing matching old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_continued_child_thread_when_five_are_already_active()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread resume: continued child thread thread-269 for issue #269 after Codex restart.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject recovery-wave continuation that would exceed the active child Codex thread limit"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity continuation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
