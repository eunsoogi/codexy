type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_binds_each_actor_to_its_branch_or_worktree_setup_action() -> TestResult {
    for (label, setup, expected) in [
        ("unrelated action before child branch setup", "The parent set requirements then the child created branch codexy/463 before classification.", false),
        ("unrelated action before parent branch setup", "The child set requirements then the parent created branch codexy/463 before classification.", true),
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
fn validator_binds_setup_relations_to_sentence_and_repeated_subject_boundaries() -> TestResult {
    for (boundary, prefix, separator) in [
        (
            "sentence",
            "The child reviewed requirements before classification.",
            " ",
        ),
        (
            "repeated child subject",
            "The child reviewed requirements before classification",
            " and ",
        ),
    ] {
        for (form, setup) in [
            ("explicit active", "The child created branch codexy/463 after classification."),
            ("explicit passive", "Branch codexy/463 was created by the child after classification."),
            ("unqualified active", "Created branch codexy/463 after classification."),
            ("unqualified passive", "Branch codexy/463 was created after classification."),
        ] {
            assert_with_classification(
                &format!("{boundary} bounds the {form} relation"),
                child_owned_classification(),
                &format!("{prefix}{separator}{setup}"),
                true,
            )?;
        }
    }
    Ok(())
}

#[test]
fn validator_retains_a_negated_clause_subject_across_and_then() -> TestResult {
    assert_with_classification(
        "and then retains the child subject for the later switch",
        parent_owned_classification(),
        "The child did not create a worktree after classification and then switched to branch codexy/463 before classification.",
        false,
    )?;
    assert_with_classification(
        "and then control preserves after-classification timing",
        child_owned_classification(),
        "The child did not create a worktree after classification and then switched to branch codexy/463 after classification.",
        true,
    )
}

#[test]
fn validator_keeps_negated_before_timing_out_of_the_setup_relation() -> TestResult {
    for (label, setup, expected) in [
        (
            "explicit active direct not before classification is not pre-classification setup",
            "The child created branch codexy/463 not before classification but after classification.",
            true,
        ),
        (
            "explicit passive direct not before classification is not pre-classification setup",
            "Branch codexy/463 was created by the child not before classification but after classification.",
            true,
        ),
        (
            "unqualified active direct not before classification is not pre-classification setup",
            "Created branch codexy/463 not before classification but after classification.",
            true,
        ),
        (
            "unqualified passive direct not before classification is not pre-classification setup",
            "Branch codexy/463 was created not before classification but after classification.",
            true,
        ),
        (
            "explicit active modifier-spanning not before classification is not pre-classification setup",
            "The child created branch codexy/463 not at any time before classification but after classification.",
            true,
        ),
        (
            "explicit passive modifier-spanning not before classification is not pre-classification setup",
            "Branch codexy/463 was created by the child not at any time before classification but after classification.",
            true,
        ),
        (
            "unqualified active modifier-spanning not before classification is not pre-classification setup",
            "Created branch codexy/463 not at any time before classification but after classification.",
            true,
        ),
        (
            "unqualified passive modifier-spanning not before classification is not pre-classification setup",
            "Branch codexy/463 was created not at any time before classification but after classification.",
            true,
        ),
        (
            "explicit active never before classification is not pre-classification setup",
            "The child created branch codexy/463 never before classification, only after classification.",
            true,
        ),
        (
            "explicit passive never before classification is not pre-classification setup",
            "Branch codexy/463 was created by the child never before classification, only after classification.",
            true,
        ),
        (
            "unqualified active never before classification is not pre-classification setup",
            "Created branch codexy/463 never before classification, only after classification.",
            true,
        ),
        (
            "unqualified passive never before classification is not pre-classification setup",
            "Branch codexy/463 was created never before classification, only after classification.",
            true,
        ),
        (
            "affirmative unqualified passive immediately before classification remains pre-classification setup",
            "Branch codexy/463 was created immediately before classification.",
            false,
        ),
        (
            "affirmative unqualified active at any time before classification remains pre-classification setup",
            "Created branch codexy/463 at any time before classification.",
            false,
        ),
    ] {
        assert_with_classification(label, child_owned_classification(), setup, expected)?;
    }
    Ok(())
}

#[test]
fn validator_recognizes_clause_bounded_timing_negation_grammar() -> TestResult {
    for (polarity, timing) in [
        ("not at any point in time", "not at any point in time before classification but after classification"),
        ("never once", "never once before classification only after classification"),
    ] {
        for (form, setup) in [
            ("explicit active", "The child created branch codexy/463"),
            ("explicit passive", "Branch codexy/463 was created by the child"),
            ("unqualified active", "Created branch codexy/463"),
            ("unqualified passive", "Branch codexy/463 was created"),
        ] {
            assert_with_classification(
                &format!("{polarity} keeps {form} setup out of pre-classification timing"),
                child_owned_classification(),
                &format!("{setup} {timing}."),
                true,
            )?;
        }
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

#[test]
fn validator_scopes_no_to_bounded_setup_noun_phrases() -> TestResult {
    for (setup, expected) in [
        ("created no new branch before classification", true),
        ("created no local branch or worktree before classification", true),
        ("created absolutely no branch before classification", true),
        ("No new branch was created by the child before classification", true),
        ("No branch was created before classification", true),
        ("created a new branch before classification", false),
        ("created branch with no branch protection before classification", false),
    ] { assert_with_classification("bounded setup noun-phrase negation", unclassified_child(), setup, expected)?; }
    Ok(())
}

pub(crate) fn assert_with_classification(label: &str, classification: &str, setup: &str, expected: bool) -> TestResult {
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

pub(crate) fn child_owned_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\nLane type: implementation\nSecondary surfaces: validators\nOwner decision: affirmative child-owned because the delegated child owns implementation\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: create branch after classification\nStop/blocker: None\nSource thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:actor-grammar\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=implement; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=463:create_goal:actor-grammar\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:actor-grammar\nPlan tool call: update_plan"
}

fn unclassified_child() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned"
}
