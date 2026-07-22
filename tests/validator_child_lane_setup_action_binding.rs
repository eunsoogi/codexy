type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_binds_each_actor_to_its_branch_or_worktree_setup_action() -> TestResult {
    for (label, setup, expected) in [
        ("unrelated action before child branch setup", "The parent set requirements then the child created branch codexy/463 before classification.", false),
        ("unrelated action after child worktree setup", "The child checked out worktree for codexy/463 after classification, then the parent set requirements.", false),
        ("parent setup then child setup fails closed", "The parent created branch codexy/parent, then the child created branch codexy/463 after classification.", false),
        ("child setup then orchestrator setup fails closed", "The child set up worktree for codexy/463; then the orchestrator set up worktree for codexy/parent.", false),
        ("two non-child setup actions remain non-child", "The parent created branch codexy/parent, then the orchestrator set up worktree for codexy/review.", true),
        ("passive child setup after unrelated action", "The parent set requirements; branch `codexy/463` was created by the child after classification.", false),
        ("passive parent then active child setup", "Worktree for codexy/parent was set up by the parent, but the child created branch codexy/463.", false),
        ("set remains unrelated while set up qualifies", "The child set expectations, then the parent set up worktree for codexy/463.", true),
        ("negated child then parent setup", "The child did not create branch codexy/463, then the parent created branch codexy/parent.", true),
        ("negated parent then child setup", "The parent did not create branch codexy/parent, then the child created branch codexy/463.", false),
        ("neutral non-setup predicates", "The child discussed branch codexy/463 and the parent reviewed the worktree plan.", true),
    ] {
        assert_with_classification(label, parent_owned_classification(), setup, expected)?;
    }
    Ok(())
}

#[test]
fn validator_scopes_negation_and_timing_to_each_setup_action() -> TestResult {
    for (label, setup, expected) in [
        ("parent before does not steal child after timing", "The parent created branch codexy/parent before classification, then the child created branch codexy/463 after classification.", true),
        ("parent after does not erase child before timing", "The parent created branch codexy/parent after classification, then the child created branch codexy/463 before classification.", false),
        ("negated child before does not erase child after", "The child did not create branch codexy/old before classification, then the child created branch codexy/463 after classification.", true),
        ("negated parent after does not erase child before", "The child created worktree for codexy/463 before classification, then the parent did not create branch codexy/parent after classification.", false),
    ] {
        assert_with_classification(label, child_owned_classification(), setup, expected)?;
    }
    Ok(())
}

#[test]
fn validator_tracks_structural_setup_relations_without_treating_plans_or_negations_as_events(
) -> TestResult {
    for (setup, classification, expected) in [
        ("The child created branch codexy/463 before classification.", child_owned_classification(), false),
        ("The child created branch codexy/463 prior to classification.", child_owned_classification(), false),
        ("The child implementation thread and the parent created branch codexy/463 before classification.", child_owned_classification(), false),
        ("The child hasn't created branch codexy/463 before classification.", unclassified_child(), true),
        ("Branch codexy/463 will be created by the child after classification.", unclassified_child(), true),
        ("The child created no branch before classification.", unclassified_child(), true),
    ] {
        assert_with_classification(
            "structural setup relation must preserve timing, coordination, polarity, and tense",
            classification,
            setup,
            expected,
        )?;
    }
    Ok(())
}

fn assert_with_classification(label: &str, classification: &str, setup: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, format!("{classification}\n{setup}"))?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(output.status.success(), expected, "{label}:\nstdout:\n{}\nstderr:\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    Ok(())
}

fn parent_owned_classification() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: parent-owned\nTask classification:\nLane type: review response\nSecondary surfaces: validators\nOwner decision: affirmative parent-owned because the parent owns orchestration\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: coordinate after classification\nStop/blocker: None"
}

fn child_owned_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\nLane type: implementation\nSecondary surfaces: validators\nOwner decision: affirmative child-owned because the delegated child owns implementation\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: create branch after classification\nStop/blocker: None\nSource thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:actor-grammar\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=implement; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=463:create_goal:actor-grammar\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:actor-grammar\nPlan tool call: update_plan"
}

fn unclassified_child() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned"
}
