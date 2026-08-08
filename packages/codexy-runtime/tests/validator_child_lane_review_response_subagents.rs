use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_subagent_used_only_for_review_response_fixes() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned review-response lane assigned to Codex worktree thread 019ef
Subthread/worktree owner: Codex worktree thread 019ef; subagent Gauss used only for review-response fixes
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject used-only-for rationale when it names review-response fixes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_subagent_used_only_for_review_response_validation() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned review-response lane assigned to Codex worktree thread 019ef
Subthread/worktree owner: Codex worktree thread 019ef; subagent Gauss used only for review-response QA validation
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow helper-only review-response validation when a true worktree thread owns the lane\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_subagent_used_only_for_review_response_qa_fixes() -> TestResult {
    for evidence in [
        r#"Owner decision: child-owned review-response lane assigned to Codex worktree thread 019ef
Subthread/worktree owner: Codex worktree thread 019ef; subagent Gauss used only for review-response QA fixes
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned review-response lane assigned to Codex worktree thread 019ef
Subthread/worktree owner: Codex worktree thread 019ef; subagent Gauss used only for review response QA fixes
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            !output.status.success(),
            "validator should reject QA-labelled review-response fixes because they imply subagent implementation ownership\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_spawn_agent_owner_denials_with_true_worktree_owner() -> TestResult {
    for evidence in [
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef; no spawn_agent owner used
Subthread/worktree owner: Codex worktree thread 019ef
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef; not assigned to spawn_agent
Subthread/worktree owner: Codex worktree thread 019ef
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            output.status.success(),
            "validator should treat spawn_agent owner denials as non-owner evidence\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_required_reviewer_gate_metadata_with_true_worktree_owner() -> TestResult {
    let output = run_ownership_validator(
        r#"Owner decision: child-owned implementation lane assigned to Codex worktree thread 019ef; reviewer gate required before handoff
Subthread/worktree owner: Codex worktree thread 019ef
Parent implementation setup: none
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow required reviewer-gate metadata when a true Codex worktree thread owns implementation\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
