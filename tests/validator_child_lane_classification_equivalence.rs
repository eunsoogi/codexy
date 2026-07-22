type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_normalizes_gfm_and_owner_decision_equivalence_classes() -> TestResult {
    let complete = complete_gfm_classification();
    let table = complete
        .split_once("Task classification:\n")
        .expect("fixture has a classification marker")
        .1;
    let incomplete = table.replacen("| Stop/blocker | None |", "", 1);
    let cases = [
        (
            "escaped GFM cell",
            complete.replacen("goal, plan", "goal \\| plan", 1),
            true,
        ),
        (
            "unescaped extra GFM column",
            complete.replacen("goal, plan", "goal | plan", 1),
            false,
        ),
        ("repeated complete table", format!("{complete}\n{table}"), true),
        ("repeated incomplete table", format!("{complete}\n{incomplete}"), false),
        (
            "documented current-thread rationale",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                1,
            ),
            true,
        ),
        (
            "contrastive parent rationale",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "affirmative current-thread-owned because parent-owned is reserved for orchestration",
                1,
            ),
            true,
        ),
        (
            "contrastive unknown rationale",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "affirmative current-thread-owned because unknown ownership remains unresolved elsewhere",
                1,
            ),
            true,
        ),
        (
            "post-boundary current-thread display text",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "affirmative current-thread-owned because the active thread is not current-thread-owned",
                1,
            ),
            true,
        ),
        (
            "ambiguous current-thread owner",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "affirmative current-thread-owned or parent-owned",
                1,
            ),
            false,
        ),
        (
            "missing current-thread rationale",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "current-thread-owned",
                1,
            ),
            false,
        ),
        (
            "parent-selected owner",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "affirmative parent-owned because the parent owns issue-sized work",
                1,
            ),
            false,
        ),
        (
            "unknown-selected owner",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "affirmative unknown ownership because the owner was not classified",
                1,
            ),
            false,
        ),
        (
            "ambiguous-selected owner",
            complete.replacen(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
                "affirmative ambiguous ownership because two owners are named",
                1,
            ),
            false,
        ),
    ];
    for (name, classification, expected) in cases {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("handoff.md");
        std::fs::write(&path, format!("{classification}\nPlan tool call: update_plan\n"))?;
        assert_eq!(
            crate::support::validator_child_lane_ownership_file(&path)?.status.success(),
            expected,
            "equivalence class: {name}"
        );
    }
    Ok(())
}

fn complete_gfm_classification() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: current-thread-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative current-thread-owned because the active thread owns issue-sized work |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}
