#[path = "structured_contract.rs"]
mod structured_contract;

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
