
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn workflow_profiles_are_exactly_versioned_and_have_one_invariant_floor() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/references/workflow-profiles.json"),
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
    for (name, evidence) in [
        ("strict requires formal evidence", "Workflow profile: strict"),
        ("light durable delegation requires formal evidence", "Workflow profile: light\nDurable delegation: yes"),
        ("standard multi-lane work requires formal evidence", "Workflow profile: standard\nMulti-lane ownership: yes"),
        ("light audit request requires formal evidence", "Workflow profile: light\nExplicit audit evidence: requested"),
    ] {
        assert_profile_result(name, evidence, false)?;
    }
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
fn ordinary_list_boundaries_preserve_only_active_formal_triggers() -> TestResult {
    for (name, evidence, expected) in [
        ("plus metadata stays active", "Workflow profile: light\n+ Task kind: security review", false),
        ("nested numeral-one metadata stays active", "Workflow profile: light\n1. 1. Task kind: security review", false),
        ("bullet ends carried inline code", "Workflow profile: light\nContext: `open\n- Task kind: security review\nclose`", false),
        ("ordered item ends carried inline code", "Workflow profile: light\nContext: `open\n1. Task kind: security review\nclose`", false),
        ("blank-separated numeral-two fenced metadata stays inactive", "Workflow profile: light\n\n2. ```text\n   Task kind: security review\n   ```", true),
        ("document-start numeral-two fenced metadata stays inactive", "2. ```text\n   Task kind: security review\n```\nWorkflow profile: light", true),
        ("document-start numeral-two plus metadata stays active", "2. + Task kind: security review", false),
        ("document-start numeral-two nested-one metadata stays active", "2. 1. Task kind: security review", false),
        ("ATX predecessor numeral-two plus metadata stays active", "# Section\n2. + Task kind: security review\nWorkflow profile: light", false),
        ("ATX predecessor numeral-two nested-one metadata stays active", "# Section\n2. 1. Task kind: security review\nWorkflow profile: light", false),
        ("ATX predecessor numeral-two fenced metadata stays inactive", "# Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light", true),
        ("ordered ATX predecessor plus metadata stays active", "2. # Section\n2. + Task kind: security review\nWorkflow profile: light", false),
        ("ordered ATX predecessor nested-one metadata stays active", "2. # Section\n2. 1. Task kind: security review\nWorkflow profile: light", false),
        ("ordered ATX predecessor fenced metadata stays inactive", "2. # Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light", true),
        ("inline-code-only predecessor plus metadata stays inactive", "`label`\n2. + Task kind: security review\nWorkflow profile: light", true),
        ("inline-code-only predecessor nested-one metadata stays inactive", "`label`\n2. 1. Task kind: security review\nWorkflow profile: light", true),
        ("inline-code-only predecessor fenced metadata stays active", "`label`\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light", false),
        ("indented-code predecessor plus metadata stays inactive", "Context\n    # Section\n2. + Task kind: security review\nWorkflow profile: light", true),
        ("indented-code predecessor nested-one metadata stays inactive", "Context\n    # Section\n2. 1. Task kind: security review\nWorkflow profile: light", true),
        ("indented-code predecessor fenced metadata stays active", "Context\n    # Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light", false),
        ("inline-code pseudo-heading plus metadata stays inactive", "`label` # Section\n2. + Task kind: security review\nWorkflow profile: light", true),
        ("inline-code pseudo-heading nested-one metadata stays inactive", "`label` # Section\n2. 1. Task kind: security review\nWorkflow profile: light", true),
        ("inline-code pseudo-heading fenced metadata stays active", "`label` # Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light", false),
        ("comment pseudo-heading plus metadata stays inactive", "<!-- note --> # Section\n2. + Task kind: security review\nWorkflow profile: light", true),
        ("comment pseudo-heading nested-one metadata stays inactive", "<!-- note --> # Section\n2. 1. Task kind: security review\nWorkflow profile: light", true),
        ("comment pseudo-heading fenced metadata stays active", "<!-- note --> # Section\n2. ```text\n   Task kind: security review\n```\nWorkflow profile: light", false),
    ] {
        assert_profile_result(name, evidence, expected)?;
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
            &format!("Workflow profile: strict\n{}", formal_classification_for(source, owner)),
            true,
        )?;
    }
    let invalid_owner = formal_classification_for("current-thread-classified", "parent-owned")
        .replacen("Lane ownership: parent-owned", "Lane ownership: unknown", 1);
    assert_profile_result(
        "strict profile rejects invalid ownership text",
        &format!("Workflow profile: strict\n{invalid_owner}"),
        false,
    )?;
    assert_profile_result(
        "parent-owned strict classification cannot authorize child setup",
        &format!(
            "Workflow profile: strict\n{}\nChild branch codexy/500-child was created after classification.",
            formal_classification_for("current-thread-classified", "parent-owned")
        ),
        false,
    )
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
        "skills/orchestration/references/workflow-profiles.json",
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
fn active_markdown_uses_matching_fence_delimiters() -> TestResult {
    assert_profile_result(
        "mismatched fence markers do not activate historical strict evidence",
        "```text\n~~~\nWorkflow profile: strict\n````\nWorkflow profile: light",
        true,
    )
}

pub(super) fn assert_profile_result(
    name: &str,
    evidence: &str,
    expected: bool,
) -> TestResult {
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
        ("current-thread-classified", "parent-owned") => "Ownership metadata source: current-thread-classified\nLane ownership: parent-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative parent-owned because the parent owns the work |\n| Atomic scope | issue-sized |\n| Required skills | orchestration |\n| Required tools/evidence | focused validation |\n| First allowed action | implement after classification |\n| Stop/blocker | None |",
        ("current-thread-classified", "external/human-owned") => "Ownership metadata source: current-thread-classified\nLane ownership: external/human-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative external/human-owned because an external owner owns the work |\n| Atomic scope | issue-sized |\n| Required skills | orchestration |\n| Required tools/evidence | focused validation |\n| First allowed action | implement after classification |\n| Stop/blocker | None |",
        ("current-thread-classified", "current-thread-owned") => "Ownership metadata source: current-thread-classified\nLane ownership: current-thread-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative current-thread-owned because the active thread owns the work |\n| Atomic scope | issue-sized |\n| Required skills | orchestration |\n| Required tools/evidence | focused validation |\n| First allowed action | implement after classification |\n| Stop/blocker | None |",
        ("parent-supplied", "child-owned") => "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative child-owned because the delegated child owns the work |\n| Atomic scope | issue-sized |\n| Required skills | orchestration |\n| Required tools/evidence | focused validation |\n| First allowed action | implement after classification |\n| Stop/blocker | None |",
        _ => panic!("test fixture only supports valid ownership records"),
    }
}
