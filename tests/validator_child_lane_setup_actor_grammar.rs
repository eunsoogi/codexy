type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_classifies_setup_actor_independently_of_voice_and_order() -> TestResult {
    for (label, setup, expected) in [
        (
            "active child branch after classification",
            "The child created branch codexy/463 after classification.",
            false,
        ),
        (
            "passive child branch after classification",
            "Branch codexy/463 was created by the child after classification.",
            false,
        ),
        (
            "passive child worktree after classification",
            "Worktree for codexy/463 was set up by the child lane after classification.",
            false,
        ),
        (
            "active child worktree before classification",
            "The owning child thread checked out worktree for codexy/463 before classification.",
            false,
        ),
        (
            "passive child branch before classification with cosmetic punctuation",
            "Branch `codexy/463` was created, by the child, before classification.",
            false,
        ),
        (
            "passive parent branch remains non-child",
            "Branch codexy/463 was created by the parent after classification.",
            true,
        ),
        (
            "passive orchestrator worktree remains non-child",
            "Worktree for codexy/463 was set up by the orchestrator after classification.",
            true,
        ),
        (
            "negated passive child branch remains absent",
            "Branch codexy/463 was not created by the child after classification.",
            true,
        ),
        (
            "negated active child worktree remains absent",
            "The child did not set up worktree for codexy/463 after classification.",
            true,
        ),
        (
            "unqualified setup remains neutral for parent ownership",
            "Branch codexy/463 was created after classification.",
            true,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}

#[test]
fn validator_resolves_setup_subject_and_agent_conflicts_by_grammatical_role() -> TestResult {
    for (label, setup, expected) in [
        (
            "active child subject governs conflicting parent adjunct",
            "The child created branch codexy/463 by the parent after classification.",
            false,
        ),
        (
            "active child subject governs conflicting orchestrator adjunct before classification",
            "The child created worktree for codexy/463, by the orchestrator, before classification.",
            false,
        ),
        (
            "active parent subject governs conflicting child adjunct",
            "The parent created branch codexy/463 by the child after classification.",
            true,
        ),
        (
            "active orchestrator subject governs conflicting child adjunct",
            "The orchestrator set up worktree for codexy/463 by the child after classification.",
            true,
        ),
        (
            "agreeing active child subject and agent",
            "The child created branch codexy/463 by the child after classification.",
            false,
        ),
        (
            "agreeing active parent subject and agent",
            "The parent created branch codexy/463 by the parent after classification.",
            true,
        ),
        (
            "passive post-action child agent",
            "Branch codexy/463 was created by the child after classification.",
            false,
        ),
        (
            "passive post-action parent agent",
            "Branch codexy/463 was created by the parent after classification.",
            true,
        ),
        (
            "passive child agent governs parent recipient",
            "The branch for the parent was created by the child before classification.",
            false,
        ),
        (
            "passive parent agent governs child recipient",
            "The branch for the child was created by the parent before classification.",
            true,
        ),
        (
            "passive pre-action child agent",
            "By the child, branch `codexy/463` was created after classification.",
            false,
        ),
        (
            "passive pre-action parent agent",
            "By the parent, branch `codexy/463` was created after classification.",
            true,
        ),
        (
            "conflicting passive agents child then parent fail closed",
            "By the child, branch codexy/463 was created by the parent after classification.",
            false,
        ),
        (
            "conflicting passive agents parent then child fail closed",
            "By the parent, branch codexy/463 was created by the child after classification.",
            false,
        ),
        (
            "child setup conflict remains isolated in a later clause",
            "The parent coordinated review; however the child created branch codexy/463 by the parent after classification.",
            false,
        ),
        (
            "parent setup control remains isolated in a later clause",
            "The child discussed review; but the parent created branch codexy/463 by the child after classification.",
            true,
        ),
        (
            "negated active child conflict remains absent",
            "The child did not create branch codexy/463 by the parent after classification.",
            true,
        ),
        (
            "negated passive child agent remains absent",
            "By the child, branch codexy/463 was not created after classification.",
            true,
        ),
        (
            "unqualified setup remains neutral",
            "Branch codexy/463 was created after classification.",
            true,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}

#[test]
fn validator_fails_closed_for_coordinated_child_setup_subjects() -> TestResult {
    for (label, setup, expected) in [
        (
            "coordinated child and parent subjects fail closed",
            "The child and the parent created branch codexy/463 before classification.",
            false,
        ),
        (
            "coordinated parent and orchestrator subjects remain non-child",
            "The parent and the orchestrator created branch codexy/463 before classification.",
            true,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}

fn assert_result(label: &str, setup: &str, expected: bool) -> TestResult {
    let evidence = format!("{}\n{setup}", parent_owned_classification());
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

fn parent_owned_classification() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: parent-owned\nTask classification:\nLane type: review response\nSecondary surfaces: validators\nOwner decision: affirmative parent-owned because the parent owns orchestration\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: coordinate after classification\nStop/blocker: None"
}
