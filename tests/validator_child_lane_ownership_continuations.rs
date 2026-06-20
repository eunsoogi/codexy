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
fn validator_allows_keyed_absent_parent_reads_in_setup_bullet()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads:
- Child reads: src/validation/hooks.rs
- Parent reads: none
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve keyed absent parent-read setup evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_setup_when_recovery_is_empty_before_stop_condition()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: created draft worktree before child delegation
Recovery:
Stop condition: disclose the workflow defect, preserve the diff, inspect user overlap, and delegate to a clean child thread
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should stop empty recovery continuations at unlisted metadata"
    );
    Ok(())
}
