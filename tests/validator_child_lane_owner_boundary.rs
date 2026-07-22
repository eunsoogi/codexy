type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_shared_owner_boundary_rejects_invalid_governing_metadata() -> TestResult {
    for owner in [
        "child-owned or parent-owned",
        "child-owned and parent-owned",
        "child-owned implementation",
        "unknown",
        "",
    ] {
        for action in [
            "Child branch codexy/463 was created before task classification.",
            "Goal tool call: create_goal",
            "Plan tool call: update_plan",
        ] {
            assert_result(
                "invalid governing owner must retain fail-closed setup context",
                &format!(
                    "Ownership metadata source: parent-supplied\nLane ownership: {owner}\n{action}"
                ),
                false,
            )?;
        }
    }
    for (owner, action) in [
        ("parent-owned", "Parent coordination branch was not created."),
        (
            "external/human-owned",
            "Workflow note: setup waits for an external owner.",
        ),
    ] {
        assert_result(
            "valid non-child owners remain neutral without child setup",
            &format!(
                "Ownership metadata source: current-thread-classified\nLane ownership: {owner}\n{action}"
            ),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn validator_shared_owner_boundary_matches_colon_and_gfm_decisions() -> TestResult {
    for gfm in [false, true] {
        for (source, authority, display, expected) in [
            ("parent-supplied", "child-owned", "child-owned", true),
            (
                "current-thread-classified",
                "current-thread-owned",
                "current-thread-owned",
                true,
            ),
            (
                "parent-supplied",
                "child-owned",
                "current-thread-owned",
                false,
            ),
            (
                "current-thread-classified",
                "current-thread-owned",
                "child-owned",
                false,
            ),
        ] {
            assert_result(
                "display owner must match governing authority in every representation",
                &format!(
                    "{}\n{}",
                    classification(source, authority, display, gfm),
                    controls()
                ),
                expected,
            )?;
        }
    }
    for metadata in [
        "Required evidence: owner matrix captured",
        "Workflow owner note: parent-owned remains orchestration-only",
    ] {
        assert_result(
            "unrelated metadata remains neutral after complete classification",
            &format!(
                "{}\n{metadata}\n{}",
                classification("parent-supplied", "child-owned", "child-owned", false),
                controls()
            ),
            true,
        )?;
    }
    Ok(())
}

fn classification(source: &str, authority: &str, display: &str, gfm: bool) -> String {
    let decision = if gfm {
        format!("affirmative {display} because the selected lane owns implementation")
    } else {
        format!("{display} implementation lane")
    };
    let fields = if gfm {
        format!(
            "| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | {decision} |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
        )
    } else {
        format!(
            "Lane type: implementation\nSecondary surfaces: validators\nOwner decision: {decision}\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: implement after classification\nStop/blocker: None"
        )
    };
    format!(
        "Ownership metadata source: {source}\nLane ownership: {authority}\nTask classification:\n{fields}"
    )
}

fn controls() -> &'static str {
    "Source thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:owner-boundary\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=implement; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=463:create_goal:owner-boundary\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:owner-boundary\nPlan tool call: update_plan\nChild branch codexy/463 was created after classification."
}

fn assert_result(name: &str, evidence: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, evidence)?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(
        output.status.success(),
        expected,
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
