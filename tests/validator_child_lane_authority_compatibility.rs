type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_enforces_source_owner_compatibility_across_all_actions() -> TestResult {
    for gfm in [false, true] {
        for source in ["parent-supplied", "current-thread-classified"] {
            for owner in [
                "parent-owned",
                "child-owned",
                "current-thread-owned",
                "external/human-owned",
            ] {
                let compatible = source == "current-thread-classified" || owner == "child-owned";
                let record = authority_record(source, owner, gfm);
                for action in [reported_goal(), "Plan tool call: update_plan"] {
                    assert_result(
                        "control compatibility",
                        &format!("{record}\n{action}"),
                        compatible,
                    )?;
                }
                for (action, explicit_child_scope) in [
                    (
                        "Branch codexy/463 was created after classification.",
                        false,
                    ),
                    (
                        "Child branch codexy/463 was created after classification.",
                        true,
                    ),
                ] {
                    let authorized_setup = matches!(owner, "child-owned" | "current-thread-owned");
                    let expected = compatible && (authorized_setup || !explicit_child_scope);
                    assert_result(
                        "setup compatibility",
                        &format!("{record}\n{action}"),
                        expected,
                    )?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn validator_fails_closed_until_a_compatible_record_is_constructed() -> TestResult {
    let table = classification_fields("child-owned", false);
    for record in [
        "Ownership metadata source: parent-supplied".to_owned(),
        "Lane ownership: child-owned\nOwnership metadata source: parent-supplied".to_owned(),
        format!("Ownership metadata source: parent-supplied\nLane ownership: unknown"),
        format!(
            "Ownership metadata source: parent supplied\nLane ownership: child-owned\nTask classification:\n{table}"
        ),
        format!(
            "Ownership metadata source: parent-supplied\nLane ownership: parent-owned\nTask classification:\n{}",
            classification_fields("parent-owned", false)
        ),
    ] {
        assert_result(
            "incomplete, malformed, ordered, and incompatible records",
            &format!("{record}\nPlan tool call: update_plan"),
            false,
        )?;
    }
    assert_result(
        "a lane reset discards the incompatible record",
        "Ownership metadata source: parent-supplied\nLane ownership: parent-owned\nTask classification:\nPR: #464\nPlan tool call: update_plan",
        true,
    )
}

fn authority_record(source: &str, owner: &str, gfm: bool) -> String {
    format!(
        "Ownership metadata source: {source}\nLane ownership: {owner}\nTask classification:\n{}",
        classification_fields(owner, gfm)
    )
}

fn classification_fields(owner: &str, gfm: bool) -> String {
    let decision = format!("affirmative {owner} because the selected lane owns this work");
    if gfm {
        format!(
            "| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | {decision} |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
        )
    } else {
        format!(
            "Lane type: implementation\nSecondary surfaces: validators\nOwner decision: {decision}\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: implement after classification\nStop/blocker: None"
        )
    }
}

fn reported_goal() -> &'static str {
    "Source thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:compatibility\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=repair; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=authority matrix; next action=create goal; transition key=463:create_goal:compatibility\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:compatibility"
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
