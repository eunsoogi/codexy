mod support;

use support::routing_validator::{
    TestResult, assert_accepted, assert_rejected, duplicate_recipient_section,
};

const ROUTES: [(&str, &str, &str, &str, &str); 2] = [
    (
        "Captured #433 parent-to-generic-child evidence",
        "gpt-5.6-terra",
        "gpt-5.6-sol",
        "child-433",
        "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
    ),
    (
        "Reverse child-to-root evidence",
        "gpt-5.6-sol",
        "gpt-5.6-terra",
        "root-433",
        "child-to-root evidence must pass recipient gpt-5.6-sol/high",
    ),
];

fn assert_omissions_rejected(prefix: &str) -> TestResult {
    for (marker, recipient, sender, thread, expected) in ROUTES {
        let metadata = format!(
            "configured_ui_model=\"{recipient}\"; actual_turn_context_model=\"{sender}\"; per_message_model=\"{recipient}\""
        );
        for arguments in [
            format!("threadId: \"{thread}\", thinking: \"high\""),
            format!("threadId: \"{thread}\", model: \"{recipient}\""),
        ] {
            assert_rejected(
                &format!(
                    "{prefix}{marker}: {metadata}; send_message_to_thread({{ {arguments} }})."
                ),
                expected,
            )?;
        }
    }
    Ok(())
}

#[test]
fn validator_rejects_plain_evidence_omissions() -> TestResult {
    assert_omissions_rejected("")
}

#[test]
fn validator_rejects_numbered_evidence_omissions() -> TestResult {
    assert_omissions_rejected("1. ")
}

#[test]
fn validator_ignores_historical_heading_evidence() -> TestResult {
    assert_accepted(duplicate_recipient_section(
        "### Historical: Captured #433 parent-to-generic-child evidence: send_message_to_thread({ threadId: \"child-433\", thinking: \"high\" }).",
    )?)
}
