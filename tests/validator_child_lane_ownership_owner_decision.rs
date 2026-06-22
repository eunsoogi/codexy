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
fn validator_rejects_parent_setup_after_routing_only_child_delegation()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: routing-only child delegation to child thread thread-148; parent remains coordination-only
Parent coordination: created issue #148, prepared branch name, and wrote handoff text
Parent implementation setup: created implementation branch codexy/148-parent-orchestrator-no-direct-edits and draft worktree after delegation
Implementation-surface reads: parent read src/validation/child_lane_ownership.rs after delegation
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent implementation setup after routing-only child delegation"
    );
    Ok(())
}

#[test]
fn validator_allows_parent_setup_for_parent_owned_owner_decision()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned (not child-owned); parent owns implementation
Parent implementation setup: created implementation branch codexy/148-parent-owned-fix and draft worktree
Implementation-surface reads: parent read src/validation/child_lane_ownership.rs
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow parent implementation setup when the owner decision is parent-owned\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_setup_after_thread_tool_discovery_owner_decision()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Parent coordination: searching for thread tools and preparing handoff text
Parent implementation setup: created implementation branch codexy/146-thread-tool-discovery
Implementation-surface reads: parent read plugins/codexy/skills/task-classification/SKILL.md
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent implementation setup after thread/worktree discovery routing"
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_setup_after_child_routing_required()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery; child routing required
Parent implementation setup: created implementation branch codexy/148-parent-orchestrator-no-direct-edits
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject parent setup when child routing is required"
    );
    Ok(())
}

#[test]
fn validator_allows_parent_owned_lane_setup_without_child_routing()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane type: orchestration/lane setup
Owner decision: parent-owned for branch/worktree setup; parent owns implementation
Parent implementation setup: created implementation branch codexy/149-parent-owned-lane
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat ordinary parent-owned lane setup as child routing\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_parent_setup_when_child_routing_is_negated()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane type: implementation
Owner decision: parent-owned for implementation setup; no child routing required
Parent implementation setup: created implementation branch codexy/148-parent-owned-followup
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat negated child routing as child routing\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_parent_setup_after_multiline_reassignment()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: routing-only child delegation to child thread thread-148; parent remains coordination-only
Maintainer reassignment:
- explicit maintainer reassignment to parent
Parent implementation setup: created implementation branch codexy/148-parent-orchestrator-no-direct-edits
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow routing-gated parent setup after multiline reassignment\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
