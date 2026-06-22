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
fn validator_allows_subagent_helper_with_true_worktree_owner()
-> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef for implementation ownership
Subthread/worktree owner: Codex worktree thread 019ef
Specialist helper: multi_agent_v1 codexy-sentinel used only for reviewer gate
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef for implementation ownership
Subthread/worktree owner: Codex worktree thread 019ef
Non-owner helper: multi_agent_v1 codexy-sentinel reviewer gate
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            output.status.success(),
            "validator should allow subagents as helpers when a true worktree thread owns implementation\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_multi_agent_rationale_with_true_worktree_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef; multi-agent not useful because the change is atomic
Subthread/worktree owner: Codex worktree thread 019ef
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow a multi-agent not-useful rationale when a true Codex worktree thread owns implementation\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_multi_agent_rationale_on_true_worktree_owner_field()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef
Subthread/worktree owner: Codex worktree thread 019ef; multi-agent not useful because the lane is atomic
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow multi-agent rationale on a thread-owner field when a true Codex worktree thread owns implementation\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_multi_agent_rationale_with_true_child_thread_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: routing-only child delegation to child thread thread-148; multi-agent not useful because atomic
Subthread/worktree owner: child thread thread-148
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow a multi-agent not-useful rationale when a true child thread owns implementation\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_parent_owned_routing_only_multi_agent_rationale()
-> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required; multi-agent not useful because atomic
Parent coordination: searching for thread tools and preparing handoff text
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for implementation; multi-agent not useful because atomic
Parent implementation setup: parent owns implementation
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            output.status.success(),
            "validator should allow parent-owned evidence with a multi-agent not-useful rationale\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_non_agent_codexy_slugs_in_non_child_owner_decisions()
-> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        r#"Owner decision: parent-owned for codexy-mcp-lsp validation
Parent implementation setup: parent owns implementation
Maintainer reassignment: none
"#,
        r#"Owner decision: current-thread-owned for codexy-mcp-codegraph validation
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            output.status.success(),
            "validator should allow non-agent Codexy slugs in non-child owner decisions\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_no_subagent_substitute_exposure_blocker()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned routing blocked because thread/worktree tools are unavailable; no subagent substitute used.
Thread/worktree tool blocker: codex_app thread tools unavailable in this session.
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow exposure blockers that explicitly deny using a subagent substitute\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
