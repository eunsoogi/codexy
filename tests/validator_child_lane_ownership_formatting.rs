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
fn validator_preserves_child_lane_across_blank_pr_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned

PR: #130
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should preserve child ownership across blank-line PR metadata"
    );
    Ok(())
}

#[test]
fn validator_allows_nested_absent_parent_authored_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Review response: parent-authored implementation commits: none; child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow nested absent parent-authored evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_processes_reassignment_value_before_ownership_words()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
PR: #130
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: explicit maintainer reassignment to parent; implementation ownership: parent
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should process explicit reassignment before treating ownership words as lane boundaries\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_orchestrator_pushed_review_response_without_reassignment()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
PR: #130
Review response: orchestrator pushed commit abc123 to fix review feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should treat orchestrator-pushed review-response commits like parent-pushed fixes"
    );
    Ok(())
}

#[test]
fn validator_preserves_child_lane_across_implementation_ownership_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation ownership: child thread
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should not end child-owned lanes on non-lane implementation ownership metadata"
    );
    Ok(())
}

#[test]
fn validator_treats_list_style_lane_ownership_as_boundary() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
- Lane ownership: parent-owned
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should recognize list-style lane ownership records as lane boundaries\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_treats_parent_owned_owner_fields_as_lane_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    for owner_field in ["Owner", "Lane owner"] {
        let output = run_ownership_validator(&format!(
            r#"PR: #1
Lane ownership: child-owned
Review response: child-authored commit abc123 fixed feedback
Maintainer reassignment: none

{owner_field}: parent-owned
Review response: parent-authored implementation commit def456 fixed feedback
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should treat `{owner_field}: parent-owned` as a lane boundary\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_keeps_pr_metadata_inside_child_owned_header() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Owner: child-thread-1
PR: #130
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should keep PR metadata inside the child-owned header"
    );
    Ok(())
}

#[test]
fn validator_keeps_pr_metadata_inside_child_owned_header_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Owner: child-thread-1
Branch: codexy/129-prevent-parent-patching
Head: 18d3102b8af7b0544e366b85deff457816da27ea
PR: #130
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should keep branch/head metadata inside the child-owned header"
    );
    Ok(())
}

#[test]
fn validator_keeps_pr_metadata_inside_child_owned_header_worktree_path()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Worktree path: /Users/eunsoo/.codex/worktrees/bdd7/codexy
PR: #130
Review response: parent-authored implementation commit abc123 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should keep worktree path metadata inside the child-owned header"
    );
    Ok(())
}
