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
fn validator_treats_child_owner_metadata_as_child_owned_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Child owner: thread-1
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should treat child owner metadata as child-owned lane evidence"
    );
    Ok(())
}

#[test]
fn validator_does_not_read_later_metadata_as_blank_reassignment()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment:
Stop condition: explicit maintainer reassignment to parent
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat later metadata as maintainer reassignment evidence"
    );
    Ok(())
}
