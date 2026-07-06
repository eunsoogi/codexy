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
fn validator_rejects_new_owner_when_and_joined_lookup_finds_linked_pr_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269 and existing owner thread thread-300 found for PR #300.
Thread creation: created child thread thread-new for issue #269 / PR #300.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should preserve and-joined found owner evidence on mixed lookup lines"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should require reuse or old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_new_owner_when_operation_uses_and_joined_pr_id()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Existing issue/PR owner check: existing owner thread thread-300 found for PR #300.
Thread creation: created child thread thread-new for issue #269 and PR #300.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should keep and-joined PR ids on child-thread operations"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should require reuse or old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_positive_raw_tool_after_negated_raw_tool_clause()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #270.
Thread tools: did not call create_thread for issue #269 and called create_thread for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should split a later positive raw-tool call after a negated raw-tool clause"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-cap raw-tool call, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
