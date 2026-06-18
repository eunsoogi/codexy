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

#[test]
fn validator_ignores_parent_fix_before_child_owned_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"PR: #1
Lane ownership: parent-owned
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
        "validator should not attribute a preceding parent-owned lane fix to the child-owned lane\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_detects_fix_before_child_owned_line_in_same_lane()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"PR: #130
Review response: parent-authored implementation commit abc123 fixed feedback
Lane ownership: child-owned
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not depend on child-owned evidence preceding the parent-authored fix"
    );
    Ok(())
}

#[test]
fn validator_preserves_pending_fix_across_owner_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"PR: #130
Review response: parent-authored implementation commit abc123 fixed feedback
Owner: child-thread-1
Lane ownership: child-owned
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not drop pending parent-authored evidence across owner metadata"
    );
    Ok(())
}

#[test]
fn validator_keeps_child_owned_lane_across_owner_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"PR: #130
Lane ownership: child-owned
Owner: child-thread-1
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not end a child-owned lane on owner metadata"
    );
    Ok(())
}

#[test]
fn validator_preserves_reassignment_before_child_owned_line()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"PR: #130
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: explicit maintainer reassignment to parent
Lane ownership: child-owned
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should carry reassignment evidence that appears before the child-owned marker\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_ignores_reassignment_before_child_owned_lane() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"PR: #1
Lane ownership: parent-owned
Maintainer reassignment: explicit maintainer reassignment to parent

PR: #2
Lane ownership: child-owned
Review response: parent-authored implementation commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not let preceding reassignment suppress a later child-owned violation"
    );
    Ok(())
}

#[test]
fn validator_ignores_parent_owned_fix_between_child_owned_lanes()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"PR: #1
Lane ownership: child-owned
Review response: child-authored commit abc123 fixed feedback
Maintainer reassignment: none

PR: #2
Lane ownership: parent-owned
Review response: parent-authored implementation commit def456 fixed feedback
Maintainer reassignment: none

PR: #3
Lane ownership: child-owned
Review response: child-authored commit ghi789 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not leak a parent-owned fix between child-owned lanes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_ignores_parent_owned_reassignment_between_child_owned_lanes()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"PR: #1
Lane ownership: child-owned
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none

PR: #2
Lane ownership: parent-owned
Maintainer reassignment: explicit maintainer reassignment to parent

PR: #3
Lane ownership: child-owned
Review response: child-authored commit ghi789 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject the first child-owned lane before parent-owned reassignment can mask it"
    );
    Ok(())
}
