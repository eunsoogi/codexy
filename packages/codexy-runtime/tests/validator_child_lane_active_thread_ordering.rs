use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_owner_check_after_child_thread_operation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Thread creation: created child thread thread-269 for issue #269.
Existing issue/PR owner check: no existing owner thread found for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject owner lookup evidence that appears after child thread creation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("before the operation"),
        "stderr should name pre-operation owner lookup evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_old_owner_disposition_after_replacement_operation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-148 found for issue #269.
Thread creation: created replacement child thread thread-269 for issue #269.
Old owner disposition: thread-148 was stopped as unusable and explicitly superseded.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject old-owner disposition evidence that appears after replacement creation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing pre-operation old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_creation_for_issue_without_matching_owner_lookup()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 2
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Active child Codex threads: 3
Thread creation: created child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject creation for #270 when only #269 had an owner lookup"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing matching owner lookup, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_mixed_reuse_and_new_issue_with_matching_lookups()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-148 found for issue #269.
Thread resume: continued child thread thread-148 for issue #269.
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #270.
Thread creation: created child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow reusing #269 owner and creating #270 after a matching no-owner lookup\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_scopes_old_owner_disposition_to_matching_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: existing owner thread thread-148 found for issue #269.
Old owner disposition: thread-148 was stopped as unusable and explicitly superseded.
Thread creation: created replacement child thread thread-269 for issue #269.
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-270-old found for issue #270.
Thread creation: created replacement child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should require old-owner disposition for the matching #270 owner, not reuse #269 disposition"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing matching old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_stale_issue_only_old_owner_disposition_for_later_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: existing owner thread thread-111 found for issue #269.
Old owner disposition: existing owner thread was stopped as unusable and explicitly superseded for issue #269.
Thread creation: created replacement child thread thread-222 for issue #269.
Active child Codex threads: 3
Existing issue/PR owner check: existing owner thread thread-222 found for issue #269.
Thread creation: created replacement child thread thread-333 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not reuse stale issue-only old-owner disposition for a later replacement"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing current old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_thread_only_creation_after_different_issue_lookup()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 2
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject unscoped thread creation when the only owner lookup is for #269"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing matching owner lookup, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_natural_issue_creation_after_different_issue_lookup()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 2
Existing issue/PR owner check: no existing owner thread found for issue #269.
Called create_thread for Codexy issue 270 lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should parse natural issue 270 operation identity and reject mismatched #269 lookup"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing matching owner lookup, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_natural_pr_creation_after_different_issue_lookup()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 2
Existing issue/PR owner check: no existing owner thread found for issue #269.
Called create_thread for Codexy PR 270 lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should parse natural PR 270 operation identity and reject mismatched #269 lookup"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing matching owner lookup, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
