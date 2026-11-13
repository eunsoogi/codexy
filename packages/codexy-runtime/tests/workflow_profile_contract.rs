// The contract requires formal ownership proof only when the active lane's risk or delegation signals demand it.
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn risk_triggers_cannot_be_downgraded_to_light_or_standard() -> TestResult {
    for (name, evidence) in [
        (
            "strict requires formal evidence",
            "Task kind: security review",
        ),
        (
            "light audit request requires formal evidence",
            "Workflow profile: light\nExplicit audit evidence: requested",
        ),
    ] {
        assert_profile_result(name, evidence, false)?;
    }
    assert_profile_result(
        "light read-only work stays lightweight",
        "Workflow profile: light\nTask kind: read-only",
        true,
    )?;
    assert_profile_result(
        "standard routine mutation stays lightweight",
        "Workflow profile: standard\nTask kind: ordinary single-owner mutation",
        true,
    )?;
    assert_profile_result(
        "the omitted profile defaults to light",
        "Task kind: documentation",
        true,
    )?;
    assert_profile_result(
        "strict work with formal evidence succeeds",
        &format!("Task kind: security review\n{}", formal_classification()),
        true,
    )?;
    assert_profile_result(
        "retired profile labels do not choose review",
        "Workflow profile: experimental",
        true,
    )
}

#[test]
fn collaboration_signals_follow_risk_without_bypassing_ownership() -> TestResult {
    let delegated_child = format!(
        "Workflow profile: light\nDurable delegation: yes\nMulti-lane ownership: yes\n{}",
        formal_classification_for("parent-supplied", "child-owned")
    );
    assert_profile_result(
        "low-risk delegated multi-lane work keeps its selected profile and owner record",
        &delegated_child,
        true,
    )?;
    assert_profile_result(
        "independent multi-lane work keeps its selected profile",
        &format!(
            "Workflow profile: standard\nMulti-lane ownership: yes\n{}",
            formal_classification()
        ),
        true,
    )?;
    for (name, evidence) in [
        (
            "bare delegation still requires complete ownership metadata",
            "Workflow profile: light\nDurable delegation: yes",
        ),
        (
            "bare multi-lane signal still requires complete ownership metadata",
            "Workflow profile: standard\nMulti-lane ownership: yes",
        ),
        (
            "partial delegated metadata still requires complete ownership metadata",
            "Workflow profile: light\nDurable delegation: yes\nLane ownership: child-owned\nPlan tool call: update_plan",
        ),
    ] {
        assert_profile_result(name, evidence, false)?;
    }
    Ok(())
}

#[test]
fn strict_formal_profiles_are_owner_neutral_but_child_setup_is_not() -> TestResult {
    for (source, owner) in [
        ("current-thread-classified", "parent-owned"),
        ("current-thread-classified", "external/human-owned"),
        ("current-thread-classified", "current-thread-owned"),
        ("parent-supplied", "child-owned"),
    ] {
        assert_profile_result(
            &format!("strict profile accepts valid {owner} classification"),
            &format!(
                "Task kind: security review\n{}",
                formal_classification_for(source, owner)
            ),
            true,
        )?;
    }
    let invalid_owner = formal_classification_for("current-thread-classified", "parent-owned")
        .replacen("Lane ownership: parent-owned", "Lane ownership: unknown", 1);
    assert_profile_result(
        "strict profile rejects invalid ownership text",
        &format!("Task kind: security review\n{invalid_owner}"),
        false,
    )?;
    assert_profile_result(
        "parent-owned strict classification cannot authorize child setup",
        &format!(
            "Task kind: security review\n{}\nChild branch codexy/500-child was created after classification.",
            formal_classification_for("current-thread-classified", "parent-owned")
        ),
        false,
    )
}

#[test]
fn workflow_profile_metadata_is_current_lane_active_markdown_and_unambiguous() -> TestResult {
    assert_profile_result(
        "numbered collaboration signal still requires ownership evidence",
        "Workflow profile: light\n1. Durable delegation: yes because the lane persists",
        false,
    )?;
    assert_profile_result(
        "security work cannot default to light evidence",
        "Task kind: security and secrets",
        false,
    )?;
    assert_profile_result(
        "retired blank profile does not select a tier",
        "Workflow profile: \nTask kind: documentation",
        true,
    )?;
    assert_profile_result(
        "duplicate profiles in one lane are rejected",
        "Workflow profile: light\nTask kind: security review",
        false,
    )?;
    assert_profile_result(
        "a current light profile is not overridden by a previous lane",
        "Task kind: security review\nReview response: previous lane closed\nWorkflow profile: light",
        true,
    )?;
    assert_profile_result(
        "fenced historical strict evidence does not override current light work",
        "```text\nTask kind: security review\n```\nWorkflow profile: light",
        true,
    )?;
    assert_profile_result(
        "a fenced historical table does not satisfy current strict proof",
        &format!(
            "Task kind: security review\n```text\n{}\n```",
            formal_classification()
        ),
        false,
    )?;
    Ok(())
}

#[test]
fn active_markdown_uses_matching_fence_delimiters() -> TestResult {
    assert_profile_result(
        "mismatched fence markers do not activate historical strict evidence",
        "```text\n~~~\nTask kind: security review\n````\nWorkflow profile: light",
        true,
    )
}

pub(super) fn assert_profile_result(name: &str, evidence: &str, expected: bool) -> TestResult {
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

pub(super) fn formal_classification() -> &'static str {
    formal_classification_for("current-thread-classified", "current-thread-owned")
}

fn formal_classification_for(source: &str, owner: &str) -> &'static str {
    match (source, owner) {
        ("current-thread-classified", "parent-owned") => {
            "Ownership metadata source: current-thread-classified\nLane ownership: parent-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative parent-owned because the parent owns the work |\n| Atomic scope | issue-sized |\n| Required skills | orchestration |\n| Required tools/evidence | focused validation |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
        }
        ("current-thread-classified", "external/human-owned") => {
            "Ownership metadata source: current-thread-classified\nLane ownership: external/human-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative external/human-owned because an external owner owns the work |\n| Atomic scope | issue-sized |\n| Required skills | orchestration |\n| Required tools/evidence | focused validation |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
        }
        ("current-thread-classified", "current-thread-owned") => {
            "Ownership metadata source: current-thread-classified\nLane ownership: current-thread-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative current-thread-owned because the active thread owns the work |\n| Atomic scope | issue-sized |\n| Required skills | orchestration |\n| Required tools/evidence | focused validation |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
        }
        ("parent-supplied", "child-owned") => concat!(
            "Source thread id: parent-workflow-profile\n",
            "Assignment objective: validate child ownership\n",
            "Success criteria: preserve valid workflow classification\n",
            "Ownership metadata source: parent-supplied\n",
            "Lane ownership: child-owned\n",
            "Task classification:\n",
            "| Field | Value |\n",
            "| --- | --- |\n",
            "| Lane type | implementation |\n",
            "| Secondary surfaces | validators |\n",
            "| Owner decision | affirmative child-owned because the delegated child owns the work |\n",
            "| Atomic scope | issue-sized |\n",
            "| Required skills | orchestration |\n",
            "| Required tools/evidence | focused validation |\n",
            "| First allowed action | implement after classification |\n",
            "| Stop/blocker | None |",
        ),
        _ => panic!("test fixture only supports valid ownership records"),
    }
}

#[path = "workflow_profile_contract/boundaries.rs"]
mod boundaries;
