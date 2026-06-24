use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
fn validator_rejects_subagent_as_child_subthread_owner() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned implementation lane assigned to subagent Gauss via multi_agent_v1.spawn_agent
Subthread/worktree owner: multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject evidence that treats a multi-agent subagent as the child subthread/worktree owner"
    );
    Ok(())
}

#[test]
fn validator_rejects_explicit_subagent_assignment_despite_substitute_denial() -> TestResult {
    for evidence in [
        r#"Owner decision: child-owned implementation lane assigned to subagent Gauss; not a subagent substitute for a Codex thread
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef; assigned to subagent Gauss; not assigned to subagent Beta
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            !output.status.success(),
            "validator should reject explicit subagent assignment even when the value also denies substitute usage"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_hyphenated_multi_agent_owner_assignment() -> TestResult {
    for evidence in [
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required; multi-agent owner Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required; owned by multi-agent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;
        assert!(
            !output.status.success(),
            "validator should reject hyphenated multi-agent owner assignments before applying routing-only exemptions\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_subagent_routes_and_negated_thread_owner_claims() -> TestResult {
    for evidence in [
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required; routed to multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: no Codex thread tools available; multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: Codex thread tools unavailable; multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: Codex thread not available; multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: Codex thread was not available; multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: multi_agent_v1 subagent Gauss instead of Codex worktree thread
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: not a Codex thread; multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: not a child thread; multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Specialist helper owner: specialist helper Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;
        assert!(
            !output.status.success(),
            "validator should reject subagent routes, negated/unavailable thread owner claims, and owner-valued helper fields\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_specialist_helper_as_child_subthread_owner() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned implementation lane assigned to specialist helper Gauss
Subthread/worktree owner: specialist helper Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject evidence that treats a specialist helper as the child subthread/worktree owner"
    );
    Ok(())
}

#[test]
fn validator_rejects_role_only_spawned_agent_owners() -> TestResult {
    for evidence in [
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: worker agent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: explorer agent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: reviewer agent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: codexy-sentinel reviewer gate
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: codexy-forge
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: codexy-pathfinder
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: Codex worktree thread 019ef; codexy-forge
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: child thread thread-148; codexy-pathfinder
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef; codexy-forge
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef; multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: Codex worktree thread 019ef; multi_agent_v1 subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: child thread thread-148; subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: reviewer agent Gauss; reviewer only
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: codexy-sentinel reviewer gate; helper only
Parent implementation setup: none
Maintainer reassignment: none
"#,
        "Owner decision: child-owned implementation lane\nSubagent owner: Gauss\n",
        "Owner decision: child-owned implementation lane\nMulti-agent owner: Gauss\n",
        "Owner decision: child-owned implementation lane\nSubthread/worktree owner:\n- multi_agent_v1 subagent Gauss\n",
        "Owner decision: child-owned implementation lane\nSubthread/worktree owner: no Codex thread tools available\n- subagent: Gauss\n",
        "Owner decision: child-owned implementation lane\nSubthread/worktree owner: no Codex thread tools available\n- multi-agent: Gauss\n",
        "Owner decision: child-owned implementation lane\nSubthread/worktree owner: no Codex thread tools available\n- specialist agent: Gauss\n",
    ] {
        let output = run_ownership_validator(evidence)?;
        assert!(
            !output.status.success(),
            "validator should reject role-only spawned agent or reviewer-gate owners\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
