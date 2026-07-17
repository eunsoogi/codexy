mod support;

use support::routing_validator::{
    TestResult, assert_accepted, assert_policy_rejected, assert_rejected,
    duplicate_recipient_section,
};

fn before_recipient(text: &str) -> TestResult<String> {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?;
    Ok(skill.replacen(
        "## Recipient Model Routing",
        &format!("{text}\n\n## Recipient Model Routing"),
        1,
    ))
}

fn with_later_matrix(policy: &str) -> TestResult<String> {
    before_recipient(&format!("## GPT-5.6 Routing Matrix\n\n{policy}"))
}

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
fn validator_rejects_later_active_routing_matrix() -> TestResult {
    assert_policy_rejected(
        with_later_matrix("- Root/orchestrator: MUST use `gpt-5.6-luna`.")?,
        "root/orchestrator must use gpt-5.6-sol",
    )
}

#[test]
fn validator_rejects_embedded_affirmative_delivery_clause() -> TestResult {
    assert_rejected(
        "Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model` and `thinking`. MUST NOT infer either from historical actual `turn_context` state, the sender, or ambient defaults. child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.",
        "gpt-5.6-sol/high",
    )
}

#[test]
fn validator_ignores_historical_heading_instructions() -> TestResult {
    assert_accepted(duplicate_recipient_section(
        "### Historical: Root/orchestrator: MUST use `gpt-5.6-luna`; child-to-root delivery MUST NOT pass `model: \"gpt-5.6-sol\"`; child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.",
    )?)
}

#[test]
fn validator_ignores_commented_and_fenced_matrix_headings() -> TestResult {
    assert_accepted(before_recipient(
        "```markdown\n## GPT-5.6 Routing Matrix\n- Root/orchestrator: MUST use `gpt-5.6-luna`.\n```\n<!--\n## GPT-5.6 Routing Matrix\n- Root/orchestrator: MUST use `gpt-5.6-luna`.\n-->",
    )?)
}
