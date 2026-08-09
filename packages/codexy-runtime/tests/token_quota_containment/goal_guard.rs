use super::{TestResult, structured_contract, structured_contract_rules};

const CANONICAL_OWNER: &str =
    "The owner MUST retain its active goal and plan while an implementation obligation remains.";
const PARENT_CHILD: &str =
    "A parent or child MUST retain its active goal and plan during a nonterminal external-gate wait while an implementation obligation remains.";
const HEADING: &str = "## Event-driven token and quota containment\n";

pub(super) fn assert_boundaries() -> TestResult {
    let rule = structured_contract_rules::ORCHESTRATION[1];
    assert!(structured_contract::Contract::markdown(&format!("{HEADING}{PARENT_CHILD}"))
        .assert_rule(rule)
        .is_ok());
    assert_missing(
        rule,
        &format!("{HEADING}MUST retain its active goal and plan during a nonterminal external-gate wait while an implementation obligation remains."),
        "subject",
    );
    assert_missing(
        rule,
        &format!("{HEADING}A parent or child MAY retain its active goal and plan during a nonterminal external-gate wait while an implementation obligation remains."),
        "modality",
    );
    assert_missing(
        rule,
        &format!("{HEADING}A parent or child MUST retain its active goal and plan during a nonterminal external-gate wait."),
        "lifecycle",
    );
    assert_missing(
        rule,
        &format!("## Different policy\n{PARENT_CHILD}"),
        "heading",
    );
    assert_missing(
        rule,
        &format!("## Historical policy\n{PARENT_CHILD}"),
        "heading",
    );
    assert!(structured_contract::Contract::markdown(CANONICAL_OWNER)
        .assert_rule(structured_contract_rules::TOKEN_CONTAINMENT[0])
        .is_ok());
    Ok(())
}

fn assert_missing(rule: structured_contract::Rule, text: &str, expected: &str) {
    let error = structured_contract::Contract::markdown(text)
        .assert_rule(rule)
        .unwrap_err();
    assert_eq!(error.missing, expected, "{text}");
}
