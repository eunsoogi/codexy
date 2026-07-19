use std::process::Output;
fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}
#[test]
fn validator_rejects_article_separated_fork_start_operations_over_cap()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Forked a child thread thread-new for issue #269.",
        "Started a child thread thread-new for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
{operation}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should detect article-separated child-thread operation: {operation}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity operation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
#[test]
fn validator_rejects_active_waiting_total_over_cap() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active/waiting child Codex threads: 6
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should treat active/waiting totals as cap counts"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("keep at most five active child Codex threads"),
        "stderr should name over-capacity active/waiting total, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
#[test]
fn validator_allows_creation_after_active_waiting_total_under_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active/waiting child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept active/waiting aggregate totals as pre-operation cap evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
#[test]
fn validator_rejects_passive_created_thread_id_over_cap() -> Result<(), Box<dyn std::error::Error>>
{
    for operation in [
        "Child thread thread-269 created for issue #269.",
        "Child thread thread-269 was created for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
{operation}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should detect passive child-thread creation with an id: {operation}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity operation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
#[test]
fn validator_allows_split_issue_pr_no_owner_lookups() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Existing issue/PR owner check: no existing owner thread found for PR #300.
Thread creation: created child thread thread-new for issue #269 / PR #300.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should aggregate split no-owner coverage for issue and PR\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
#[test]
fn validator_allows_lower_count_after_comma_freed_capacity()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #268.
Thread creation: created child thread thread-268 for issue #268.
Child thread thread-268 finished, Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should reset the projection after comma-separated freed capacity\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
#[test]
fn validator_rejects_replacement_after_unrelated_freed_capacity()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Child thread thread-other stopped and freed replacement capacity.
Active child Codex threads: 5, including thread-old for issue #269.
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old was stopped as unusable for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not spend unrelated freed capacity on the old owner replacement"
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
fn validator_rejects_conjunction_separated_active_waiting_counts_over_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3 and waiting child Codex threads: 3
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should split conjunction-separated active/waiting counts"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("keep at most five active child Codex threads"),
        "stderr should name over-capacity active/waiting count, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
#[test]
fn validator_rejects_labeled_comma_child_thread_creations_over_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Existing issue/PR owner check: no existing owner thread found for issue #270.
Thread creation: created child thread thread-269 for issue #269, Thread creation: created child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should split labeled comma-separated child-thread operations"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity operation, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
#[test]
fn validator_rejects_comma_separated_disposition_for_different_issue()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269, Old owner disposition: issue #270 was stopped, Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not borrow owner identity across comma-separated disposition clauses"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing matching old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
