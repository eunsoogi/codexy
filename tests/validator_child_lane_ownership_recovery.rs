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
fn validator_rejects_negated_parent_setup_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: created draft worktree before child delegation
Recovery: did not disclose the workflow defect, cleaned up the draft worktree, inspected user overlap, and delegated to a clean child thread
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject negated parent setup recovery evidence"
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_read_in_setup_bullet() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads:
- parent read src/validation/hooks.rs
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent setup reads in bullet continuation"
    );
    Ok(())
}

#[test]
fn validator_allows_parent_substring_in_child_read_path() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads: child read src/parent_setup.rs; no parent reads
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow parent substring in a child-read path\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_parent_substring_in_child_read_bullet_path()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads:
- child read src/parent_setup.rs; no parent reads
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow parent substring in a child-read bullet path\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_parent_read_substring_inside_child_path()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads: child read docs/grandparent reads.md; no parent reads
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow parent reads substring inside a child-read path\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
