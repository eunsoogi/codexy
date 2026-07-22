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

fn complete_gfm_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | current-thread-owned child implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}
