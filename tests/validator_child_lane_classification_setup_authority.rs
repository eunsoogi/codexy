type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_non_authoritative_classification_metadata_before_control() -> TestResult {
    let valid = "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | current-thread-owned child implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |";
    for classification in [
        valid.replacen("Ownership metadata source: parent-supplied\n", "", 1),
        valid.replacen("Ownership metadata source: parent-supplied", "Ownership metadata source: parent supplied", 1),
        valid.replacen("Task classification:\n| Field | Value |\n| --- | --- |\n", "| Field | Value |\n| --- | --- |\n", 1),
        valid.replacen("| Field | Value |\n| --- | --- |\n", "", 1),
        valid.replacen("Ownership metadata source: parent-supplied", "- Ownership metadata source: parent-supplied", 1),
        valid.replacen("Lane ownership: child-owned", "Lane ownership: parent-owned", 1),
        valid.replacen("Lane ownership: child-owned", "Lane ownership: unknown", 1),
        valid.replacen("| --- | --- |", "| | |", 1),
        valid.replacen("| --- | --- |", "| - | - |", 1),
    ] {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("handoff.md");
        std::fs::write(&path, format!("{classification}\nPlan tool call: update_plan\n"))?;
        assert!(!crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    }
    Ok(())
}

#[test]
fn validator_does_not_inherit_gfm_classification_across_lanes() -> TestResult {
    let complete = complete_gfm_classification().to_owned();
    let missing_separator = complete.replacen("| --- | --- |\n", "", 1);
    for table in [&complete, &missing_separator] {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("handoff.md");
        std::fs::write(
            &path,
            format!(
                "PR: #1\n{table}\nReview response: child-authored commit abc123\nPR: #2\nPlan tool call: update_plan\n"
            ),
        )?;
        assert!(crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    }
    Ok(())
}

#[test]
fn validator_allows_current_thread_classified_child_owned_authority() -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    let classification = complete_gfm_classification()
        .replacen("parent-supplied", "current-thread-classified", 1);
    std::fs::write(&path, format!("{classification}\nPlan tool call: update_plan\n"))?;

    assert!(crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    Ok(())
}

#[test]
fn validator_allows_aligned_gfm_classification_delimiter() -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    let classification = complete_gfm_classification().replacen("| --- | --- |", "| :--- | ---: |", 1);
    std::fs::write(&path, format!("{classification}\nPlan tool call: update_plan\n"))?;

    assert!(crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    Ok(())
}

#[test]
fn validator_rejects_current_thread_owned_actions_before_classification() -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(
        &path,
        "Ownership metadata source: current-thread-classified\nLane ownership: current-thread-owned\nGoal tool call: create_goal\nPlan tool call: update_plan\nChild branch codexy/463 was created before task classification.\n",
    )?;

    assert!(!crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    Ok(())
}

#[test]
fn validator_allows_complete_current_thread_owned_classification() -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    let classification = complete_gfm_classification()
        .replacen("parent-supplied", "current-thread-classified", 1)
        .replacen("child-owned", "current-thread-owned", 1);
    std::fs::write(&path, format!("{classification}\nGoal tool call: create_goal\nPlan tool call: update_plan\nChild branch codexy/463 was created after classification.\n"))?;

    assert!(crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    Ok(())
}

#[test]
fn validator_rejects_a_later_classification_without_authority() -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    let table = complete_gfm_classification()
        .split_once("Task classification:\n")
        .expect("fixture has a classification marker")
        .1;
    std::fs::write(
        &path,
        format!("{}\nTask classification:\n{table}\nPlan tool call: update_plan\n", complete_gfm_classification()),
    )?;

    assert!(!crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    Ok(())
}

#[test]
fn validator_rejects_metadata_free_required_tool_aliases() -> TestResult {
    for alias in ["Required tools", "Required evidence"] {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("handoff.md");
        let classification = complete_gfm_classification()
            .replacen("Ownership metadata source: parent-supplied\nLane ownership: child-owned\n", "", 1)
            .replacen("Required tools/evidence", alias, 1);
        std::fs::write(&path, format!("{classification}\nPlan tool call: update_plan\n"))?;

        assert!(!crate::support::validator_child_lane_ownership_file(&path)?.status.success());
    }
    Ok(())
}

fn complete_gfm_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | current-thread-owned child implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}
