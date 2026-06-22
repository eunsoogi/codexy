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
fn validator_allows_parent_setup_when_natural_child_routing_negation_is_required()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane type: orchestration/lane setup
Owner decision: parent-owned for thread/worktree tool discovery only; no child routing is required
Parent implementation setup: created implementation branch codexy/148-parent-owned-followup
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat natural child routing negation as child routing\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_tool_search_false_blocker_when_thread_events_exist()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: no codex_app namespace and no true thread tools exposed.
App-server observed events: fresh child lane emitted thread/start and turn/start.
Blocker: thread tools are absent because tool_search missed them.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject false thread-tool blockers when real thread events exist"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("thread/start"),
        "stderr should name the false blocker evidence, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_negated_tool_search_blocker_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: no codex_app namespace and no true thread tools exposed.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
No blocker: thread tools are not absent because tool_search missed them; route through the real thread surface.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow evidence that negates a tool_search-only blocker"
    );
    Ok(())
}

#[test]
fn validator_rejects_codex_cli_thread_fallback_claim() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Thread fallback: used codex exec as a substitute for missing thread tools.
Thread requirement: satisfied by codex app-server fallback.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject Codex CLI fallback claims for thread requirements"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Codex CLI"),
        "stderr should name the forbidden fallback, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_negated_codex_cli_fallback_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Forbidden route: did not use codex exec, codex fork, or codex app-server fallback as a substitute.
Thread evidence: app-server-observed thread/start and turn/start events from a fresh child lane.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow evidence that explicitly negates forbidden CLI fallback use"
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
