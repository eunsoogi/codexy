use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn workflow_profiles_are_exactly_versioned_and_have_one_invariant_floor() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/workflow-profiles.json"),
    )?)?;

    assert_eq!(contract["version"], 1);
    assert_eq!(contract["defaultProfile"], "light");
    assert_eq!(contract["escalationOrder"], serde_json::json!(["light", "standard", "strict"]));
    assert_eq!(contract["profiles"].as_object().map(|profiles| profiles.len()), Some(3));
    assert_eq!(contract["profiles"]["light"]["includes"], serde_json::json!([
        "read-only",
        "documentation",
        "tiny fixes",
        "ordinary single-owner mutations"
    ]));
    assert_eq!(contract["profiles"]["standard"]["includes"], serde_json::json!(["non-trivial single-owner work"]));
    assert_eq!(contract["profiles"]["strict"]["requiresFormalEvidence"], true);
    assert_eq!(contract["proofAndReview"]["strict"], "formal current-head proof and the applicable Sentinel review");

    let triggers = contract["formalEvidenceTriggers"]
        .as_array()
        .ok_or("formal evidence triggers must be an array")?;
    for trigger in ["strict", "durable delegation", "multi-lane ownership", "explicit audit evidence"] {
        assert!(triggers.iter().any(|value| value == trigger), "missing {trigger}");
    }
    for invariant in [
        "destructive-action safety",
        "user and unrelated change preservation",
        "no force push or force-with-lease",
        "current-head readiness proof",
        "every governed file at or below 250 LOC with no exceptions",
    ] {
        assert!(contract["invariantFloor"].as_array().is_some_and(|items| {
            items.iter().any(|value| value == invariant)
        }), "missing invariant {invariant}");
    }
    assert_eq!(contract["authorizedMergeGates"], serde_json::json!([
        "title", "label", "thread", "connector", "merge-message"
    ]));
    Ok(())
}

#[test]
fn formal_triggers_cannot_be_downgraded_to_light_or_standard() -> TestResult {
    assert_profile_result("strict requires formal evidence", "Workflow profile: strict", false)?;
    assert_profile_result(
        "light durable delegation requires formal evidence",
        "Workflow profile: light\nDurable delegation: yes",
        false,
    )?;
    assert_profile_result(
        "standard multi-lane work requires formal evidence",
        "Workflow profile: standard\nMulti-lane ownership: yes",
        false,
    )?;
    assert_profile_result(
        "light audit request requires formal evidence",
        "Workflow profile: light\nExplicit audit evidence: requested",
        false,
    )?;
    assert_profile_result("light read-only work stays lightweight", "Workflow profile: light\nTask kind: read-only", true)?;
    assert_profile_result(
        "standard routine mutation stays lightweight",
        "Workflow profile: standard\nTask kind: ordinary single-owner mutation",
        true,
    )?;
    assert_profile_result("the omitted profile defaults to light", "Task kind: documentation", true)?;
    assert_profile_result(
        "strict work with formal evidence succeeds",
        &format!("Workflow profile: strict\n{}", formal_classification()),
        true,
    )?;
    assert_profile_result("unknown profiles fail closed", "Workflow profile: experimental", false)
}

#[test]
fn workflow_profile_metadata_is_current_lane_active_markdown_and_unambiguous() -> TestResult {
    assert_profile_result(
        "numbered trigger with rationale cannot downgrade light work",
        "Workflow profile: light\n1. Durable delegation: yes because the lane persists",
        false,
    )?;
    assert_profile_result(
        "security work cannot default to light evidence",
        "Task kind: security and secrets",
        false,
    )?;
    assert_profile_result(
        "blank profile is rejected instead of defaulting to light",
        "Workflow profile: \nTask kind: documentation",
        false,
    )?;
    assert_profile_result(
        "duplicate profiles in one lane are rejected",
        "Workflow profile: light\nWorkflow profile: strict",
        false,
    )?;
    assert_profile_result(
        "a current light profile is not overridden by a previous lane",
        "Workflow profile: strict\nReview response: previous lane closed\nWorkflow profile: light",
        true,
    )?;
    assert_profile_result(
        "fenced historical strict evidence does not override current light work",
        "```text\nWorkflow profile: strict\n```\nWorkflow profile: light",
        true,
    )?;
    assert_profile_result(
        "a fenced historical table does not satisfy current strict proof",
        &format!("Workflow profile: strict\n```text\n{}\n```", formal_classification()),
        false,
    )?;
    Ok(())
}

#[test]
fn workflow_profile_contract_rejects_extra_or_contradictory_structure() -> TestResult {
    let contract_path = std::path::Path::new(
        "skills/codex-orchestration/references/workflow-profiles.json",
    );
    for (field, value) in [
        (
            "formalEvidenceTriggers",
            serde_json::json!(["strict", "durable delegation", "multi-lane ownership", "explicit audit evidence", "extra"]),
        ),
        (
            "authorizedMergeGates",
            serde_json::json!(["title", "label", "thread", "connector", "merge-message", "extra"]),
        ),
    ] {
        let (_temp, plugin_root) =
            crate::support::copy_plugin_fixture_with_mutable_files(&[contract_path])?;
        let path = plugin_root.join(contract_path);
        let mut contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        contract[field] = value;
        std::fs::write(&path, serde_json::to_string(&contract)?)?;
        assert!(!crate::support::validator(&plugin_root, "--check")?.status.success());
    }
    Ok(())
}

#[test]
fn strict_signals_are_complete_and_negation_aware() -> TestResult {
    for task_kind in [
        "destructive mutation",
        "security and secrets",
        "permission change",
        "secret rotation",
        "release work",
        "publication workflow",
        "high-consequence external-state mutation",
        "high-risk mutation",
        "high-risk guardrail",
        "multi-lane coordination",
        "merge-sensitive change",
    ] {
        assert_profile_result(
            "strict task kind requires formal proof",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    for task_kind in [
        "no security concerns",
        "non-security documentation",
        "not a release",
        "without secret material",
    ] {
        assert_profile_result(
            "negated strict signal remains lightweight",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn strict_signal_negation_is_category_local() -> TestResult {
    for task_kind in [
        "no security but release work",
        "without permission; publication workflow",
        "not a release, but security review",
    ] {
        assert_profile_result(
            "an affirmative category after a delimiter requires strict proof",
            &format!("Task kind: {task_kind}"),
            false,
        )?;
    }
    assert_profile_result(
        "one negation may cover coordinated categories in its clause",
        "Task kind: no security or release work",
        true,
    )
}

#[test]
fn strict_signals_require_exact_category_tokens() -> TestResult {
    for task_kind in ["secretary notes", "secretarial work"] {
        assert_profile_result(
            "a category prefix inside another token remains lightweight",
            &format!("Task kind: {task_kind}"),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn active_markdown_uses_matching_fence_delimiters() -> TestResult {
    assert_profile_result(
        "mismatched fence markers do not activate historical strict evidence",
        "```text\n~~~\nWorkflow profile: strict\n````\nWorkflow profile: light",
        true,
    )
}

fn assert_profile_result(name: &str, evidence: &str, expected: bool) -> TestResult {
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

fn formal_classification() -> &'static str {
    "Ownership metadata source: current-thread-classified\nLane ownership: current-thread-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative current-thread-owned because the active thread owns the work |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | focused validation |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}
