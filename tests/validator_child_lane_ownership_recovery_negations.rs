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
fn validator_rejects_not_checked_overlap_inspection_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: created draft worktree before child delegation
Recovery: disclosed the defect, cleaned up the worktree, user overlap not checked, and delegated to a clean child thread
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject recovery evidence with overlap not checked"
    );
    Ok(())
}

#[test]
fn validator_rejects_not_reviewed_overlap_inspection_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: created draft worktree before child delegation
Recovery: disclosed the defect, cleaned up the worktree, user overlap not reviewed, and delegated to a clean child thread
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject recovery evidence with overlap not reviewed"
    );
    Ok(())
}
