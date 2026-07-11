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
fn validator_allows_creation_after_count_with_trailing_completion_then_refresh()
-> Result<(), Box<dyn std::error::Error>> {
    for completion in [
        "child thread thread-old finished and was removed",
        "PR #236 completed after merge",
        "PR #236 archived after merge",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #268.
Thread creation: created child thread thread-268 for issue #268.
Active child Codex threads: 5, {completion}
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should preserve trailing freed capacity `{completion}` before the refreshed count\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_creation_after_count_with_trailing_completion_without_refresh()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5, PR #236 completed after merge
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should still project from the parsed full count until a refreshed count appears"
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
fn validator_preserves_segment_level_completion_across_no_count_segments()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #268.
Thread creation: created child thread thread-268 for issue #268.
Child thread thread-old finished; no blockers.
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve segment-level freed capacity across later no-count text\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_preserves_completion_with_trailing_no_blockers_status()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #268.
Thread creation: created child thread thread-268 for issue #268.
Child thread thread-old finished with no blockers.
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should scope freed-capacity negation to the completion claim, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_preserves_verb_first_removed_thread_as_freed_capacity()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #268.
Thread creation: created child thread thread-268 for issue #268.
Removed child thread thread-101 from the active ledger; Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should treat verb-first removal claims as freed capacity, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_treats_deleted_child_thread_as_freed_capacity()
-> Result<(), Box<dyn std::error::Error>> {
    for completion in [
        "Child thread thread-268 deleted after completion; Active child Codex threads: 4",
        "Deleted child thread thread-268 after completion; Active child Codex threads: 4",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #268.
Thread creation: created child thread thread-268 for issue #268.
{completion}
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should treat deleted child thread wording as freed capacity for `{completion}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_creation_after_generic_completion_text_after_count()
-> Result<(), Box<dyn std::error::Error>> {
    for completion in [
        "verification completed",
        "verification for PR #236 completed",
        "review for PR #236 completed",
        "tests for issue #236 completed",
        "verification for thread-old completed",
        "review for thread-old completed",
        "child thread verification completed",
        "review for child thread thread-old completed",
        "tests for child thread thread-old completed",
        "Issue #269 completed tests",
        "Thread thread-r1 completed review",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #268.
Thread creation: created child thread thread-268 for issue #268.
Active child Codex threads: 5, {completion}
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should not treat generic completion wording `{completion}` as freed capacity"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity creation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
