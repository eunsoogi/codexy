use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_reused_owner_lookup_from_previous_operation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269-a for issue #269.
Active child Codex threads: 4
Thread creation: created child thread thread-269-b for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not reuse owner lookup evidence captured before a previous same-issue child operation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing current owner lookup evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_reused_lookup_when_matching_operations_are_separated()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 2
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269-a for issue #269.
Active child Codex threads: 3
Existing issue/PR owner check: no existing owner thread found for issue #270.
Thread creation: created child thread thread-270-a for issue #270.
Active child Codex threads: 4
Thread creation: created child thread thread-269-b for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should bound owner lookups by the last prior operation for the same issue"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
        "stderr should name missing current owner lookup evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
