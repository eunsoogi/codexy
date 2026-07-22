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
    assert_allowed(&format!(
        "{}\nChild branch codexy/231-branch-classification-guard was created after classification; no child branch was created before task classification.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        complete_child_classification_table(),
    ))
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
    assert_allowed(&format!(
        "{}\nWorktree for codexy/231-branch-classification-guard was created after task classification.\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        complete_child_classification_table(),
    ))
}

#[test]
fn validator_rejects_child_goal_before_task_classification() -> TestResult {
    assert_rejected(&format!(
        "Lane ownership: child-owned\n{}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        child_goal_call()
    ))
}

#[test]
fn validator_rejects_child_plan_before_task_classification() -> TestResult {
    assert_rejected(
        "Lane ownership: child-owned\nPlan tool call: update_plan\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
    )
}

#[test]
fn validator_allows_each_new_lane_classification_before_goal_and_plan() -> TestResult {
    for classification in [
        complete_child_classification_table(),
        complete_current_thread_classification_table(),
    ] {
        assert_allowed(&format!(
            "{}\n{classification}\n{}\nPlan tool call: update_plan\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
            complete_child_classification(), child_goal_call()
        ))?;
    }
    Ok(())
}

#[test]
fn validator_allows_parent_goal_and_plan_before_later_complete_child_lane() -> TestResult {
    assert_allowed(&format!(
        "{}\nGoal tool call: create_goal\nPlan tool call: update_plan\n{}\n{}\nPlan tool call: update_plan\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
        complete_parent_classification(),
        complete_child_classification(),
        child_goal_call()
    ))
}

#[test]
fn validator_rejects_later_child_goal_and_plan_before_its_classification() -> TestResult {
    for (boundary, control) in [
        ("1. Lane ownership: child-owned", "5. Plan tool call: update_plan"),
        ("- Lane ownership: child-owned", "- Plan tool call: update_plan"),
        ("- [ ] Lane ownership: child-owned", "- [ ] Plan tool call: update_plan"),
    ] {
        assert_rejected(&format!(
            "{}\n{boundary}\n{control}\n{}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n",
            complete_child_classification(), complete_child_classification()
        ))?;
    }
    Ok(())
}

#[test]
fn validator_rejects_numbered_or_bulleted_control_before_classification() -> TestResult {
    for control in [
        child_goal_call().replace(
            "Goal tool call: create_goal",
            "5. Goal tool call: create_goal",
        ),
        "- Plan tool call: update_plan".to_owned(),
        "* Plan tool call: update_plan".to_owned(),
        "- [ ] Plan tool call: update_plan".to_owned(),
        child_goal_call().replace(
            "Goal tool call: create_goal",
            "1. [x] Goal tool call: create_goal",
        ),
    ] {
        assert_rejected(&format!(
            "Lane ownership: child-owned\n{control}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
        ))?;
    }
    Ok(())
}

#[test]
fn validator_rejects_prefixed_child_ownership_and_control_before_classification() -> TestResult {
    for control in [
        "5. Goal tool call: create_goal",
        "- Plan tool call: update_plan",
        "- [ ] Plan tool call: update_plan",
    ] {
        assert_rejected(&format!(
            "1. Lane ownership: child-owned\n{control}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
        ))?;
    }
    Ok(())
}

fn complete_parent_classification() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: parent-owned\nTask classification:\nLane type: validation\nSecondary surfaces: validators\nOwner decision: affirmative parent-owned because the parent owns orchestration\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: validate evidence\nStop/blocker: None"
}

fn complete_child_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative child-owned because the delegated child owns implementation |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}

fn complete_child_classification_table() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative child-owned because the delegated child owns implementation |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}

fn complete_current_thread_classification_table() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: current-thread-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative current-thread-owned because the active thread owns issue-sized work |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}

fn child_goal_call() -> &'static str {
    "Source thread id: parent-335\nGoal control state: source_thread_id=parent-335\nGoal transition key: 335:create_goal:classification\nParent goal pre-delivery: operation=create_goal; parent task=parent-335; delivery=confirmed; task surface=codex task/thread; issue=#335; plan step=implement; branch=codexy/335; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=335:create_goal:classification\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-335; delivery=confirmed; task surface=codex task/thread; transition key=335:create_goal:classification"
}
