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

#[test]
fn validator_rejects_embedded_evidence_omissions() -> TestResult {
    for (marker, recipient, sender, thread, expected) in ROUTES {
        assert_rejected(
            &embedded_evidence(marker, recipient, sender, thread, false),
            expected,
        )?;
    }
    Ok(())
}

#[test]
fn validator_accepts_complete_embedded_evidence() -> TestResult {
    for (marker, recipient, sender, thread, _) in ROUTES {
        assert_accepted(duplicate_recipient_section(&embedded_evidence(
            marker, recipient, sender, thread, true,
        ))?)?;
    }
    assert_accepted(duplicate_recipient_section(
        "- Historical report quotes Every `send_message_to_thread` call and Captured #433 parent-to-generic-child evidence: send_message_to_thread({ threadId: \"child-433\" }).",
    )?)
}

fn embedded_evidence(
    marker: &str,
    recipient: &str,
    sender: &str,
    thread: &str,
    complete: bool,
) -> String {
    let thinking = complete
        .then_some(", thinking: \"high\"")
        .unwrap_or_default();
    format!(
        "- Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model` and `thinking`. MUST NOT infer either from historical actual `turn_context` state, the sender, or ambient defaults. {marker}: configured_ui_model=\"{recipient}\"; actual_turn_context_model=\"{sender}\"; per_message_model=\"{recipient}\"; send_message_to_thread({{ threadId: \"{thread}\", model: \"{recipient}\"{thinking} }})."
    )
}
