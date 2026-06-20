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

#[test]
fn validator_rejects_parent_implementation_setup_without_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: created branch codexy/135-enforce-codegraph-lsp and draft worktree before child delegation
Implementation-surface reads: parent read src/validation/hooks.rs and tests/validator_hooks.rs
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent implementation setup artifacts in a child-owned lane"
    );
    Ok(())
}

#[test]
fn validator_rejects_hyphenated_parent_created_implementation_branches()
-> Result<(), Box<dyn std::error::Error>> {
    for setup in [
        "Parent-created implementation branch codexy/135-enforce-codegraph-lsp",
        "Orchestrator-created implementation branch codexy/135-enforce-codegraph-lsp",
    ] {
        let output = run_ownership_validator(&format!(
            "Lane ownership: child-owned\n{setup}\nMaintainer reassignment: none\n"
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject `{setup}` as parent implementation setup"
        );
    }
    Ok(())
}

#[test]
fn validator_allows_child_implementation_surface_reads() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Child implementation-surface reads: child read src/validation/hooks.rs
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow child implementation-surface reads\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_absent_parent_created_setup_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    for setup in [
        "Parent-created implementation branch: none",
        "No parent-created implementation branch",
        "Orchestrator-created implementation worktree: none",
        "No orchestrator-created implementation worktree",
    ] {
        let output = run_ownership_validator(&format!(
            "Lane ownership: child-owned\n{setup}\nMaintainer reassignment: none\n"
        ))?;

        assert!(
            output.status.success(),
            "validator should allow absent setup evidence `{setup}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_parent_coordination_for_child_owned_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent coordination: created issue #136, prepared branch name, and wrote handoff text
Child owner: thread-136
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow coordination-only parent evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
