#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_artifacts.rs"]
mod structured_contract_artifacts;

use structured_contract::{Contract, Modality, Rule};

#[test]
fn structured_rules_accept_equivalent_wording_and_reject_semantic_drift() {
    let rule = Rule::new(
        "heartbeat.waiting.no-persistent-goal",
        "owner",
        Modality::Prohibited,
        &["retain", "goal"],
        &["heartbeat", "waiting"],
    );
    for equivalent in [
        "The owner must not retain a goal while heartbeat waiting.",
        "The owner must never keep a goal throughout heartbeat waiting.",
        "The owner is required not to keep a goal during heartbeat waiting.",
    ] {
        assert!(Contract::markdown(equivalent).assert_rule(rule).is_ok());
    }

    for changed in [
        "The owner may retain a goal while heartbeat waiting.",
        "The owner must retain a goal while heartbeat waiting.",
        "The worker must not retain a goal while heartbeat waiting.",
        "The worker tells the owner they must not retain a goal while heartbeat waiting.",
        "The owner must not fail to retain a goal while heartbeat waiting.",
        "The owner must not retain a goal while not heartbeat waiting.",
        "If available, the owner must not retain a goal while heartbeat waiting.",
        "The owner must not retain a goal while ordinary work continues.",
    ] {
        let error = Contract::markdown(changed).assert_rule(rule).unwrap_err();
        assert_eq!(error.rule_id, "heartbeat.waiting.no-persistent-goal");
    }
}

#[test]
fn structured_rules_ignore_historical_headings_and_require_the_named_scope() {
    let rule = Rule::new(
        "delegation.assignment.nonrecursive",
        "helper",
        Modality::Required,
        &["include"],
        &["reviewer", "prohibition"],
    )
    .under_heading("current policy");
    let historical = Contract::markdown(
        "## Historical Example\nA helper must include a reviewer prohibition.\n\n## Current Policy\nA helper must include a reviewer prohibition.",
    );
    assert!(historical.assert_rule(rule).is_ok());
    let missing_scope =
        Contract::markdown("## Current Policy\nA helper must include a reviewer instruction.");
    assert_eq!(
        missing_scope.assert_rule(rule).unwrap_err().missing,
        "scope"
    );

    let wrong_heading =
        Contract::markdown("## Not Current Policy\nA helper must include a reviewer prohibition.");
    assert_eq!(
        wrong_heading.assert_rule(rule).unwrap_err().missing,
        "heading"
    );
}

#[test]
fn structured_rules_preserve_parent_heading_and_lifecycle_state() {
    let rule = Rule::new(
        "heartbeat.waiting.no-persistent-goal",
        "owner",
        Modality::Prohibited,
        &["retain", "goal"],
        &["heartbeat"],
    )
    .under_heading("goal lifecycle")
    .in_lifecycle(&["waiting"]);
    let active = Contract::markdown(
        "## Goal Lifecycle\n### Non-Historical Requirements\nThe owner must not retain a goal while heartbeat waiting.",
    );
    assert!(active.assert_rule(rule).is_ok());

    let wrong_state = Contract::markdown(
        "## Goal Lifecycle\n### Current Policy\nThe owner must not retain a goal after heartbeat completion.",
    );
    assert_eq!(
        wrong_state.assert_rule(rule).unwrap_err().missing,
        "lifecycle"
    );
}

#[test]
fn structured_rules_reject_conditional_examples_and_contextual_subject_confusion() {
    let rule = Rule::new(
        "heartbeat.route.binds-observation",
        "heartbeat route",
        Modality::Required,
        &["bind", "observation"],
        &["automation id"],
    );
    for changed in [
        "When available, the heartbeat route must bind the observation to its automation id.",
        "If unavailable, the heartbeat route must bind the observation to its automation id.",
        "The heartbeat route describes setup, but the fallback route must bind the observation to its automation id.",
    ] {
        assert!(Contract::markdown(changed).assert_rule(rule).is_err());
    }

    let owner_rule = Rule::new(
        "heartbeat.owner.no-goal",
        "owner",
        Modality::Prohibited,
        &["retain", "goal"],
        &["heartbeat"],
    );
    assert!(
        Contract::markdown("The owner's worker must not retain a goal during heartbeat waiting.")
            .assert_rule(owner_rule)
            .is_err()
    );

    let prompt_rule = Rule::new(
        "prompt.you.use-skill",
        "you",
        Modality::Required,
        &["use"],
        &["orchestration"],
    );
    assert!(
        Contract::markdown_for_subject("The worker must use orchestration during handoff.", "you")
            .assert_rule(prompt_rule)
            .is_err()
    );
}

#[test]
fn structured_rules_ignore_fenced_examples_and_accept_not_created_lifecycle() {
    let example_rule = Rule::new(
        "delegation.current.helper-prohibition",
        "helper",
        Modality::Required,
        &["include"],
        &["reviewer", "prohibition"],
    )
    .under_heading("current policy");
    let fenced = Contract::markdown(
        "## Current Policy\n```text\nA helper must include a reviewer prohibition.\n```",
    );
    assert!(fenced.assert_rule(example_rule).is_err());

    let lifecycle_rule = Rule::new(
        "heartbeat.not-created.no-goal",
        "owner",
        Modality::Prohibited,
        &["retain", "goal"],
        &["heartbeat"],
    )
    .in_lifecycle(&["not-created"]);
    assert!(
        Contract::markdown(
            "The owner must not retain a goal during the not-created heartbeat lifecycle."
        )
        .assert_rule(lifecycle_rule)
        .is_ok()
    );
}

#[test]
#[should_panic(expected = "token.prompt.no-polling-language")]
fn forbidden_artifact_concepts_fail_with_their_rule_id() {
    structured_contract_artifacts::TextShape::new("The prompt keeps polling forever.")
        .assert_absent_concepts("token.prompt.no-polling-language", &["polling"]);
}

#[test]
fn forbidden_artifact_inflections_reject_poll_forms_without_prefix_false_positives() {
    for word in [
        "poll",
        "polls",
        "polled",
        "poller",
        "pollers",
        "polling",
        "repoll",
        "repolls",
        "repolled",
        "repoller",
        "repollers",
        "repolling",
    ] {
        let result = std::panic::catch_unwind(|| {
            structured_contract_artifacts::TextShape::new(word)
                .assert_absent_inflections("token.prompt.no-polling-language", &["poll"]);
        });
        assert!(result.is_err(), "accepted {word}");
    }
    for safe in ["pollution", "pollinate", "pollen"] {
        structured_contract_artifacts::TextShape::new(safe)
            .assert_absent_inflections("token.prompt.no-polling-language", &["poll"]);
    }
}
