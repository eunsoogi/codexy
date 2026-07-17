mod support;

use support::routing_validator::{
    TestResult, assert_accepted, assert_rejected, duplicate_recipient_section,
};

#[test]
fn validator_rejects_plain_tier_assignments() -> TestResult {
    for (policy, expected) in [
        (
            "Root/orchestrator: MUST use `gpt-5.6-luna`.",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "Generic implementation child: MUST use `gpt-5.6-sol` with `reasoning_effort: \"high\"`.",
            "generic child thread must explicitly request gpt-5.6-terra/high",
        ),
    ] {
        assert_rejected(policy, expected)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_embedded_negated_delivery_clause() -> TestResult {
    assert_rejected(
        "Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model` and `thinking`. MUST NOT infer either from historical actual `turn_context` state, the sender, or ambient defaults. child-to-root delivery MUST NOT pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
        "gpt-5.6-sol/high",
    )
}

#[test]
fn validator_ignores_historical_heading_instructions() -> TestResult {
    assert_accepted(duplicate_recipient_section(
        "### Historical: Root/orchestrator: MUST use `gpt-5.6-luna`; child-to-root delivery MUST NOT pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
    )?)
}
