type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_requires_an_affirmative_current_thread_owner_assertion() -> TestResult {
    let complete = complete_gfm_classification()
        .replacen("parent-supplied", "current-thread-classified", 1)
        .replacen("child-owned", "current-thread-owned", 1);
    let cases = [
        (
            "documented because rationale",
            "current-thread-owned because the active thread owns issue-sized work",
            true,
        ),
        (
            "contrastive parent rationale",
            "current-thread-owned because parent-owned is reserved for orchestration",
            true,
        ),
        (
            "contrastive unknown rationale",
            "current-thread-owned because unknown ownership remains unresolved elsewhere",
            true,
        ),
        (
            "contrastive ambiguous rationale",
            "current-thread-owned because ambiguous ownership is recorded for another lane",
            true,
        ),
        (
            "canonical implementation assertion",
            "current-thread-owned child implementation lane",
            true,
        ),
        (
            "leading owner negation",
            "not current-thread-owned because the active thread owns issue-sized work",
            false,
        ),
        (
            "selected owner denial",
            "current-thread-owned not allowed to act",
            false,
        ),
        (
            "missing rationale boundary",
            "current-thread-owned active issue work",
            false,
        ),
        ("empty because rationale", "current-thread-owned because", false),
        (
            "malformed owner token",
            "current-thread-ownedness because the active thread owns issue-sized work",
            false,
        ),
        (
            "parent owner selection",
            "parent-owned because the parent owns issue-sized work",
            false,
        ),
        (
            "unknown owner selection",
            "unknown ownership because the owner was not classified",
            false,
        ),
        (
            "ambiguous owner selection",
            "ambiguous ownership because two owners are named",
            false,
        ),
    ];
    let controls = "Goal tool call: create_goal\nPlan tool call: update_plan\nChild branch codexy/463 was created after classification.";
    for (name, owner, expected) in cases {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("handoff.md");
        let classification = complete.replacen(
            "current-thread-owned child implementation lane",
            owner,
            1,
        );
        std::fs::write(&path, format!("{classification}\n{controls}\n"))?;

        let output = crate::support::validator_child_lane_ownership_file(&path)?;
        assert_eq!(
            output.status.success(),
            expected,
            "{name}: goal/plan/branch controls\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn complete_gfm_classification() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | current-thread-owned child implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}
