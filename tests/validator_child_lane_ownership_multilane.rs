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
fn validator_scopes_reassignment_to_each_child_owned_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"PR: #1
Lane ownership: child-owned
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none

PR: #2
Lane ownership: child-owned
Review response: parent-authored implementation commit def456 fixed feedback
Maintainer reassignment: explicit maintainer reassignment to parent
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let reassignment for one lane suppress another lane's violation"
    );
    Ok(())
}

#[test]
fn validator_allows_multiple_reassigned_child_owned_lanes() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"PR: #1
Lane ownership: child-owned
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: explicit maintainer reassignment to parent

PR: #2
Lane ownership: child-owned
Review response: parent-authored implementation commit def456 fixed feedback
Maintainer reassignment: explicit maintainer reassignment to parent
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow each child-owned lane with its own explicit reassignment\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
