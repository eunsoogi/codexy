use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
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
fn validator_rejects_child_setup_before_task_classification() -> TestResult {
    for setup_evidence in "Child created implementation branch before starting.|Child created implementation branch codexy/228-reject-generic-reviewer-gate-sentinel-proof immediately after thread rename.|Child branch codexy/228-reject-generic-reviewer-gate-sentinel-proof was created immediately after thread rename.|Branch codexy/228-reject-generic-reviewer-gate-sentinel-proof was created immediately after thread rename.|Branch `codexy/228-reject-generic-reviewer-gate-sentinel-proof` was created immediately after thread rename.|Child branch and worktree were created before task classification.|Created child branch codexy/228-reject-generic-reviewer-gate-sentinel-proof before task classification.|Branch creation occurred before formal classification output.|Child branch creation occurred before formal classification output.|Child ran git worktree add -b codexy/228-reject-generic-reviewer-gate-sentinel-proof before task classification.|Ran git worktree add -b codexy/228-reject-generic-reviewer-gate-sentinel-proof before task classification.|Child thread created draft worktree immediately after thread rename.|Child worktree for codexy/228-reject-generic-reviewer-gate-sentinel-proof was created immediately after thread rename.".split('|') {
        for owner_evidence in
            "Lane ownership: child-owned|Owner: child-owned|Lane owner: child-owned|Child owner: codex thread 123|Owner decision: child-owned implementation lane|Owner decision: current-thread-owned child implementation lane".split('|')
        {
            let output = run_ownership_validator(&format!(
                r#"{owner_evidence}
Thread title: renamed to Codexy #228 generic reviewer gate
{setup_evidence}
Task classification:
Lane type: implementation
Secondary surfaces: workflow, validators
Owner decision: affirmative current-thread-owned because the current thread owns implementation
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#
            ))?;

            assert!(
                !output.status.success(),
                "validator should reject child branch/worktree setup before formal task classification evidence: {owner_evidence}; {setup_evidence}"
            );
        }
    }
    Ok(())
}
#[test]
fn validator_allows_child_setup_after_complete_task_classification() -> TestResult {
    assert_allowed(&format!(
        "{}\nChild created implementation branch codexy/231-branch-classification-guard after classification.\n{}",
        complete_child_classification(),
        ownership_footer()
    ))?;
    assert_allowed(&format!(
        "{}\nBranch codexy/parent-owned was created after classification.\n{}\n{}\nChild branch codexy/231-branch-classification-guard was created after classification.\n{}",
        complete_parent_classification(),
        ownership_footer(),
        complete_child_classification(),
        ownership_footer()
    ))?;
    assert_allowed(&format!(
        "{}\nChild branch codexy/231-branch-classification-guard was created after classification.\n{}",
        complete_current_thread_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_rejects_child_setup_after_incomplete_task_classification() -> TestResult {
    assert_rejected(
        r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: implementation
Owner decision: current-thread-owned child implementation lane
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: create branch after classification
Stop/blocker: None
Child branch codexy/231-branch-classification-guard was created after classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_child_setup_after_unrelated_parent_classification() -> TestResult {
    for owner_decision in [
        "parent-owned for thread/worktree tool discovery only; child routing required",
        "parent-owned orchestration; child thread/worktree owner required after setup",
    ] {
        assert_rejected(&format!(
            r#"Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
Lane type: validation
Secondary surfaces: workflow, validators
Owner decision: {owner_decision}
Atomic scope: issue-sized
Required skills: task-classification, codex-orchestration, git-workflow
Required tools/evidence: goal, plan, codegraph, LSP, Sentinel
First allowed action: validate delegated child output
Stop/blocker: None
Child branch codexy/228-reject-generic-reviewer-gate-sentinel-proof was created immediately after thread rename.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
        ))?;
    }
    Ok(())
}

#[test]
fn validator_allows_absent_child_setup_wording() -> TestResult {
    assert_allowed(
        r#"Lane ownership: child-owned
No child branch was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_present_child_setup_after_absent_child_setup_clause() -> TestResult {
    assert_rejected(
        r#"Lane ownership: child-owned
No child branch was created, but child worktree was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;
    assert_rejected(
        r#"Lane ownership: child-owned
No child branch was created before task classification and child worktree was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;
    assert_rejected(
        r#"Lane ownership: child-owned
No child branch was created before task classification. Child worktree was created before task classification.
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )
}

#[test]
fn validator_rejects_child_switch_before_task_classification() -> TestResult {
    for setup_evidence in [
        "Child switched to implementation branch codexy/231-branch-classification-guard before task classification.",
        "Child thread checked out implementation branch codexy/231-branch-classification-guard before task classification.",
        "Owning child thread checked out implementation branch codexy/231-branch-classification-guard before task classification.",
        "Child branch setup before task classification.",
        "Child worktree setup before task classification.",
        "Child set up worktree before task classification.",
        "git worktree add -b codexy/231-branch-classification-guard ../codexy-231 before task classification.",
        "git worktree add -b `codexy/231-branch-classification-guard` ../codexy-231 before task classification.",
        "git worktree add -b \"codexy/231-branch-classification-guard\" ../codexy-231 before task classification.",
        "git worktree add -b 'codexy/231-branch-classification-guard' ../codexy-231 before task classification.",
        "Branch: codexy/231-branch-classification-guard was created before task classification.",
    ] {
        assert_rejected(&format!(
            r#"Lane ownership: child-owned
{setup_evidence}
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#
        ))?;
    }
    Ok(())
}
#[test]
fn validator_allows_child_setup_after_classification_with_prior_absence() -> TestResult {
    assert_allowed(&format!(
        "{}\nNo child branch was created before task classification, but child worktree was created after classification.\n{}",
        complete_child_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_rejects_later_child_setup_before_classification() -> TestResult {
    assert_rejected(&format!(
        "{}\nChild created implementation branch codexy/231-branch-classification-guard after classification.\nChild worktree was created before task classification evidence completed.\n{}",
        complete_child_classification(),
        ownership_footer()
    ))
}

#[test]
fn validator_rejects_affirmative_child_setup_with_unrelated_no_wording() -> TestResult {
    for setup_evidence in [
        "Child branch codexy/231-branch-classification-guard was created before task classification with no delay.",
        "Child branch codexy/231-branch-classification-guard was created with no formal task-classification evidence.",
    ] {
        assert_rejected(&format!(
            r#"Lane ownership: child-owned
{setup_evidence}
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#
        ))?;
    }
    Ok(())
}

#[test]
fn validator_rejects_child_setup_claimed_before_classification_after_block() -> TestResult {
    for setup_evidence in [
        "Child branch codexy/231-branch-classification-guard was created before task classification evidence completed.",
        "Child branch codexy/231-branch-classification-guard was created before task-classification evidence completed.",
        "Child branch codexy/231-branch-classification-guard was created before formal $task-classification evidence completed.",
        "Child branch codexy/231-branch-classification-guard was created before formal `$task-classification` evidence completed.",
        "Branch codexy/231-branch-classification-guard was created before formal classification output in the child thread.",
    ] {
        assert_rejected(&format!(
            "{}\n{setup_evidence}\n{}",
            complete_child_classification(),
            ownership_footer()
        ))?;
    }
    Ok(())
}

fn complete_child_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\nLane type: implementation\nSecondary surfaces: workflow, validators\nOwner decision: affirmative child-owned because the delegated child owns implementation\nAtomic scope: issue-sized\nRequired skills: task-classification, codex-orchestration, git-workflow\nRequired tools/evidence: goal, plan, codegraph, LSP, Sentinel\nFirst allowed action: run git worktree add -b codexy/231-branch-classification-guard after classification\nStop/blocker: None"
}

fn complete_current_thread_classification() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: current-thread-owned\nTask classification:\nLane type: implementation\nSecondary surfaces: workflow, validators\nOwner decision: affirmative current-thread-owned because the current thread owns issue #231\nAtomic scope: issue-sized\nRequired skills: task-classification, codex-orchestration, git-workflow\nRequired tools/evidence: goal, plan, codegraph, LSP, Sentinel\nFirst allowed action: create branch after classification\nStop/blocker: None"
}

fn complete_parent_classification() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: parent-owned\nTask classification:\nLane type: validation\nSecondary surfaces: workflow, validators\nOwner decision: affirmative parent-owned because the parent owns orchestration\nAtomic scope: issue-sized\nRequired skills: task-classification, codex-orchestration, git-workflow\nRequired tools/evidence: goal, plan, codegraph, LSP, Sentinel\nFirst allowed action: create branch after classification\nStop/blocker: None"
}

fn ownership_footer() -> &'static str {
    "Review response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
}
