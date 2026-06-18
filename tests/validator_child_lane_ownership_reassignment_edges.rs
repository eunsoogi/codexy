use std::process::Command;

fn run_validator(evidence: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?)
}

#[test]
fn validator_rejects_not_yet_granted_reassignment() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_validator(
        "Lane ownership: child-owned\n\
         Review response: parent-authored implementation commit abc123 fixed feedback.\n\
         Maintainer reassignment: explicit maintainer reassignment to parent not yet granted\n",
    )?;

    assert!(
        !output.status.success(),
        "validator should reject not-yet-granted reassignment evidence"
    );
    Ok(())
}

#[test]
fn validator_allows_reassignment_notes_key() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_validator(
        "Lane ownership: child-owned\n\
         Review response: parent-authored implementation commit abc123 fixed feedback.\n\
         Maintainer reassignment notes: explicit maintainer reassignment to parent\n",
    )?;

    assert!(
        output.status.success(),
        "validator should accept reassignment notes as affirmative evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
