use crate::support;

use support::routing_validator::{
    TestResult, assert_accepted, assert_rejected, duplicate_recipient_section,
};

const ROUTES: [(&str, &str, &str, &str, &str, &str); 2] = [
    (
        "Captured #433 parent-to-generic-child evidence",
        "gpt-5.6-terra",
        "gpt-5.6-sol",
        "child-433",
        "high",
        "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
    ),
    (
        "Reverse child-to-root evidence",
        "gpt-5.6-sol",
        "gpt-5.6-terra",
        "root-433",
        "medium",
        "child-to-root evidence must pass recipient gpt-5.6-sol/medium",
    ),
];

fn assert_omissions_rejected(prefix: &str) -> TestResult {
    for (marker, recipient, sender, thread, effort, expected) in ROUTES {
        let metadata = format!(
            "configured_ui_model=\"{recipient}\"; actual_turn_context_model=\"{sender}\"; per_message_model=\"{recipient}\""
        );
        for arguments in [
            format!("threadId: \"{thread}\", thinking: \"{effort}\""),
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
    for (marker, recipient, sender, thread, effort, expected) in ROUTES {
        assert_rejected(
            &embedded_evidence(marker, recipient, sender, thread, effort, false),
            expected,
        )?;
    }
    Ok(())
}

#[test]
fn validator_accepts_complete_embedded_evidence() -> TestResult {
    for (marker, recipient, sender, thread, effort, _) in ROUTES {
        assert_accepted(duplicate_recipient_section(&embedded_evidence(
            marker, recipient, sender, thread, effort, true,
        ))?)?;
    }
    assert_accepted(duplicate_recipient_section(
        "- Historical report quotes Every `send_message_to_thread` call and Captured #433 parent-to-generic-child evidence: send_message_to_thread({ threadId: \"child-433\" }).",
    )?)
}

#[test]
fn validator_rejects_wrong_child_to_root_evidence_effort() -> TestResult {
    for effort in ["high", "low"] {
        assert_rejected(
            &embedded_evidence(
                "Reverse child-to-root evidence",
                "gpt-5.6-sol",
                "gpt-5.6-terra",
                "root-433",
                effort,
                true,
            ),
            "child-to-root evidence must pass recipient gpt-5.6-sol/medium",
        )?;
    }
    Ok(())
}

#[test]
fn validator_checks_case_insensitive_evidence_markers() -> TestResult {
    let markers = [
        "captured #433 parent-to-generic-child evidence",
        "rEvErSe ChIlD-tO-rOoT eViDeNcE",
    ];
    for ((_, recipient, sender, thread, effort, expected), marker) in
        ROUTES.into_iter().zip(markers)
    {
        assert_rejected(
            &embedded_evidence(marker, recipient, sender, thread, effort, false),
            expected,
        )?;
        assert_accepted(duplicate_recipient_section(&embedded_evidence(
            marker, recipient, sender, thread, effort, true,
        ))?)?;
    }
    Ok(())
}

#[test]
fn validator_bounds_case_insensitive_evidence_records() -> TestResult {
    let first = embedded_record(markers(0), 0, true);
    let second = embedded_record(markers(1), 1, false);
    assert_rejected(
        &format!("{} {first} {second}", instruction_start()),
        ROUTES[1].5,
    )?;

    let first = embedded_record(markers(0), 0, false);
    let second = embedded_record(markers(1), 1, true);
    assert_rejected(
        &format!("{} {first} {second}", instruction_start()),
        ROUTES[0].5,
    )
}

fn embedded_evidence(
    marker: &str,
    recipient: &str,
    sender: &str,
    thread: &str,
    effort: &str,
    complete: bool,
) -> String {
    let thinking = complete
        .then(|| format!(", thinking: \"{effort}\""))
        .unwrap_or_default();
    format!(
        "{} {marker}: configured_ui_model=\"{recipient}\"; actual_turn_context_model=\"{sender}\"; per_message_model=\"{recipient}\"; send_message_to_thread({{ threadId: \"{thread}\", model: \"{recipient}\"{thinking} }}).",
        instruction_start()
    )
}

fn instruction_start() -> &'static str {
    "- Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model` and `thinking`. MUST NOT infer either from historical actual `turn_context` state, the sender, or ambient defaults."
}

fn markers(index: usize) -> &'static str {
    [
        "captured #433 parent-to-generic-child evidence",
        "rEvErSe ChIlD-tO-rOoT eViDeNcE",
    ][index]
}

fn embedded_record(marker: &str, route: usize, complete: bool) -> String {
    let (_, recipient, sender, thread, effort, _) = ROUTES[route];
    let thinking = complete
        .then(|| format!(", thinking: \"{effort}\""))
        .unwrap_or_default();
    format!(
        "{marker}: configured_ui_model=\"{recipient}\"; actual_turn_context_model=\"{sender}\"; per_message_model=\"{recipient}\"; send_message_to_thread({{ threadId: \"{thread}\", model: \"{recipient}\"{thinking} }})."
    )
}
