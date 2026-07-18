use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
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
fn validator_allows_repeated_field_negative_parent_implementation_setup()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: no parent implementation setup
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow repeated-field negative parent setup evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_recovered_parent_implementation_setup() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: created draft worktree before child delegation
Recovery: disclosed the workflow defect, cleaned up the draft worktree, inspected user and other-agent overlap, and delegated to a clean child thread before implementation resumed
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow parent setup evidence with explicit recovery\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_authored_fix_despite_setup_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: created draft worktree before child delegation
Review response: parent-authored implementation commit abc123 fixed feedback
Recovery: disclosed the workflow defect, cleaned up the draft worktree, inspected user and other-agent overlap, and delegated to a clean child thread before implementation resumed
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let setup recovery suppress parent-authored implementation fixes"
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
fn validator_allows_child_reads_with_negated_parent_reads() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads: child read src/validation/hooks.rs; no parent reads
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow child reads with negated parent reads\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_child_reads_with_later_parent_read() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads: child read src/validation/hooks.rs; no parent reads, parent read src/validation/hooks.rs
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject later parent reads after a negated parent-read clause"
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
