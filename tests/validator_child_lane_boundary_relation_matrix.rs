type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_requires_fresh_classification_after_review_lane_boundaries() -> TestResult {
    for boundary in review_boundaries() {
        for action in [
            "Plan tool call: update_plan",
            "The child created branch codexy/463 after classification.",
        ] {
            assert_result(
                "a completed pre-boundary classification must not authorize the next lane",
                &format!("{}\n{boundary}\n{action}", complete_colon_classification()),
                false,
            )?;
            assert_result(
                "an incomplete pre-boundary classification remains fail closed",
                &format!("{}\n{boundary}\n{action}", incomplete_classification()),
                false,
            )?;
            assert_result(
                "a fresh complete classification authorizes the next lane",
                &format!(
                    "{}\n{boundary}\n{}\n{action}",
                    complete_colon_classification(),
                    complete_gfm_classification()
                ),
                true,
            )?;
            assert_result(
                "a fresh incomplete classification remains fail closed",
                &format!(
                    "{}\n{boundary}\n{}\n{action}",
                    complete_colon_classification(),
                    incomplete_classification()
                ),
                false,
            )?;
        }
    }
    Ok(())
}

#[test]
fn validator_uses_one_setup_action_vocabulary_for_collection_and_attribution() -> TestResult {
    for setup in [
        "The child did create branch codexy/463 before classification.",
        "The child created branch codexy/463 before classification.",
        "Branch codexy/463 was created by the child before classification.",
        "The child did switch to branch codexy/463 before classification.",
        "The child switched to branch codexy/463 before classification.",
        "Branch codexy/463 was switched by the child before classification.",
        "The child did checkout worktree codexy/463 before classification.",
        "The child checked out worktree codexy/463 before classification.",
        "Worktree codexy/463 was checked out by the child before classification.",
        "The child did setup worktree codexy/463 before classification.",
        "The child set up worktree codexy/463 before classification.",
        "Worktree codexy/463 was set up by the child before classification.",
    ] {
        assert_result(
            "every affirmative child setup form must be collected and attributed",
            &format!("{}\n{setup}", complete_colon_classification()),
            false,
        )?;
    }
    for control in [
        "The parent did create branch codexy/463 before classification.",
        "The orchestrator did switch to worktree codexy/463 before classification.",
        "The child did not create branch codexy/463 before classification.",
        "The child discussed branch codexy/463 before classification.",
        "The parent created branch codexy/parent, then the child created branch codexy/463 after classification.",
    ] {
        assert_result(
            "actor, polarity, action, and timing controls must remain scoped",
            &format!("{}\n{control}", complete_colon_classification()),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn validator_distinguishes_completed_setup_events_from_plans_and_nouns() -> TestResult {
    let unclassified_child =
        "Ownership metadata source: parent-supplied\nLane ownership: child-owned";
    for (setup, expected) in [
        (
            "The child will create branch codexy/463 after classification.",
            true,
        ),
        (
            "Branch creation requirements for codexy/463 follow classification.",
            true,
        ),
        (
            "The child did create branch codexy/463 before classification.",
            false,
        ),
        (
            "The child added worktree for codexy/463 before classification.",
            false,
        ),
        (
            "The child did add worktree for codexy/463 before classification.",
            false,
        ),
        (
            "The child will switch to branch codexy/463 after classification.",
            true,
        ),
        (
            "The child switched to branch codexy/463 before classification.",
            false,
        ),
    ] {
        assert_result(
            "only an affirmative completed setup event requires prior classification",
            &format!("{unclassified_child}\n{setup}"),
            expected,
        )?;
    }
    Ok(())
}

#[test]
fn validator_preserves_table_and_adjacent_lane_boundaries() -> TestResult {
    assert_result(
        "classification table text is not a lane boundary",
        &format!(
            "{}\nPlan tool call: update_plan",
            complete_gfm_classification().replace(
                "| Secondary surfaces | validators |",
                "| Secondary surfaces | review response: validators |"
            )
        ),
        true,
    )?;
    assert_result(
        "a later pull request boundary supersedes review-lane state",
        &format!(
            "{}\nReview response: prior lane complete\nPR: #482\nPlan tool call: update_plan",
            complete_colon_classification()
        ),
        true,
    )?;
    assert_result(
        "a pull request boundary requires fresh classification for explicit child setup",
        &format!(
            "{}\nPR: #482\nThe child created branch codexy/463 before classification.",
            complete_colon_classification()
        ),
        false,
    )?;
    assert_result(
        "adjacent review boundaries still require a fresh classification",
        &format!(
            "{}\nReview response: prior lane complete\nMaintainer reassignment: child owns repair\nPlan tool call: update_plan",
            complete_colon_classification()
        ),
        false,
    )?;
    assert_result(
        "a fresh parent-owned lane does not inherit child setup authority",
        &format!(
            "{}\nReview response: prior lane complete\n{}\nThe parent created branch codexy/review after classification.",
            complete_colon_classification(),
            complete_parent_classification()
        ),
        true,
    )
}

fn review_boundaries() -> [&'static str; 2] {
    [
        "Review response: prior review cycle completed",
        "Maintainer reassignment: explicit reassignment to child",
    ]
}

fn complete_colon_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\nLane type: implementation\nSecondary surfaces: validators\nOwner decision: affirmative child-owned because the delegated child owns implementation\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: implement after classification\nStop/blocker: None"
}

fn complete_gfm_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative child-owned because the delegated child owns implementation |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}

fn complete_parent_classification() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: parent-owned\nTask classification:\nLane type: review response\nSecondary surfaces: validators\nOwner decision: affirmative parent-owned because the parent owns orchestration\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: coordinate after classification\nStop/blocker: None"
}

fn incomplete_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\nLane type: implementation"
}

fn assert_result(label: &str, evidence: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, evidence)?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(
        output.status.success(),
        expected,
        "{label}:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
