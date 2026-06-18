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
fn validator_ignores_unlabeled_parent_fix_before_later_child_owned_pr()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"PR: #1
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none

PR: #2
Lane ownership: child-owned
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not attribute an unlabeled prior PR parent fix to a later child-owned PR\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_treats_new_pr_after_header_only_child_lane_as_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"PR: #1
Lane ownership: child-owned

PR: #2
Review response: parent-authored implementation commit abc123 fixed feedback
Lane ownership: parent-owned
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not carry header-only child ownership into the next parent-owned PR\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
