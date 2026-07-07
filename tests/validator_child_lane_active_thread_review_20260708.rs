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
fn validator_rejects_same_line_disposition_after_replacement_operation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: existing owner thread thread-old found for issue #269; Thread creation: created replacement child thread thread-new for issue #269; Old owner disposition: existing owner thread thread-old was stopped as unusable for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not credit old-owner disposition that appears after replacement operation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("old owner"),
        "stderr should name missing pre-operation old-owner disposition, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_over_cap_non_prefixed_thread_id_list_without_total()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 019ef, 019ff, 019aa, 019bb, 019cc, 019dd
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should count non-prefixed Codex thread IDs in active ledgers"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("keep at most five active child Codex threads"),
        "stderr should name over-cap non-prefixed ledger, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
