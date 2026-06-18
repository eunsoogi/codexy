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
fn validator_allows_absent_parent_authored_review_response()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Review response: no parent-authored implementation commits; child fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow review-response evidence that explicitly denies parent-authored fixes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_contradictory_parent_authored_review_response()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Review response: no parent-authored implementation commits; parent-authored review-response commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject contradictory parent-authored review-response evidence"
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_authored_fix_with_unrelated_no_value()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Review response: parent-authored implementation commit abc123; no tests run
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent-authored fixes even when the same line mentions unrelated no-values"
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_authored_field_with_unrelated_no_value()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent-authored implementation commits: parent-authored implementation commit abc123; no tests run
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent-authored field values even when they mention unrelated no-values"
    );
    Ok(())
}
