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
fn validator_allows_absent_subagent_owner_metadata_with_true_worktree_owner() -> TestResult {
    for evidence in [
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef
Subthread/worktree owner: Codex worktree thread 019ef
Subagent owner: none
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef
Subthread/worktree owner: Codex worktree thread 019ef
Multi-agent owner: none
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            output.status.success(),
            "validator should allow absent subagent-owner metadata when a true worktree thread owns implementation\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_owner_field_with_unavailable_thread_tools_and_no_subagent_substitute()
-> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned routing blocked because thread/worktree tools are unavailable
Subthread/worktree owner: none; codex thread tools unavailable; no subagent substitute used
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow owner-field evidence that denies any subagent substitute while reporting unavailable thread tools\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_bare_subagent_owner_with_unrelated_denial_marker() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner: subagent Gauss; codexy-sentinel reviewer gate not the owner
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject a bare subagent owner even when another clause denies a different owner\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
