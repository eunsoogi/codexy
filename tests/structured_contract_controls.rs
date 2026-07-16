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
    let equivalent = Contract::markdown(
        "## Goal Lifecycle\nThe owner must not retain a goal while heartbeat waiting.",
    );
    assert!(equivalent.assert_rule(rule).is_ok());

    for changed in [
        "The owner may retain a goal while heartbeat waiting.",
        "The owner must retain a goal while heartbeat waiting.",
        "The worker must not retain a goal while heartbeat waiting.",
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
}
