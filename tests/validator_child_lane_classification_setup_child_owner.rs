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

fn assert_rejected(evidence: &str) -> TestResult {
    assert!(!run_ownership_validator(evidence)?.status.success());
    Ok(())
}

fn assert_allowed(evidence: &str) -> TestResult {
    assert!(run_ownership_validator(evidence)?.status.success());
    Ok(())
}

#[test]
fn validator_rejects_child_owner_setup_without_task_classification() -> TestResult {
    for owner_evidence in [
        "Child owner: codex thread 123",
        "- Lane ownership: child-owned",
    ] {
        assert_rejected(&format!(
            r#"{owner_evidence}
Child branch codexy/228-reject-generic-reviewer-gate-sentinel-proof was created immediately after thread rename.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
        ))?;
    }
    Ok(())
}

#[test]
fn validator_allows_absent_child_owner_metadata_without_child_lane_context() -> TestResult {
    assert_allowed(
        r#"Child owner: none assigned yet
Child branch codexy/231-branch-classification-guard was created immediately after thread rename.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_allows_absent_before_classification_clause_after_valid_setup() -> TestResult {
    assert_allowed(
        r#"Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification; no child branch was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_codexy_worktree_setup_before_classification() -> TestResult {
    assert_rejected(
        r#"Lane ownership: child-owned
Worktree for codexy/231-branch-classification-guard was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_current_thread_owner_setup_before_classification() -> TestResult {
    assert_rejected(
        r#"Owner decision: current-thread-owned implementation lane for #231
Child branch codexy/231-branch-classification-guard was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_git_switch_create_before_classification() -> TestResult {
    assert_rejected(
        r#"Lane ownership: child-owned
Child ran git switch -c codexy/231-branch-classification-guard before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_unqualified_setup_before_classification() -> TestResult {
    for setup in [
        "Created branch before task classification.",
        "Created worktree before task classification.",
        "Worktree was created before task classification.",
    ] {
        assert_rejected(&format!(
            "Lane ownership: child-owned\n{setup}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
        ))?;
    }
    Ok(())
}

#[test]
fn validator_allows_codexy_worktree_setup_after_classification() -> TestResult {
    assert_allowed(
        r#"Lane ownership: child-owned
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Worktree for codexy/231-branch-classification-guard was created after task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}
