mod support;

use support::routing_validator::{
    TestResult, assert_accepted, assert_policy_rejected, assert_rejected,
    duplicate_recipient_section,
};

#[test]
fn validator_rejects_parent_assignment_with_model_only_in_later_paragraph() -> TestResult {
    assert_rejected(
        "- Parent-to-generic-child delivery MUST pass `thinking: \"high\"`.\n\nUnrelated prose uses `model: \"gpt-5.6-terra\"`.",
        "gpt-5.6-terra/high",
    )
}

#[test]
fn validator_rejects_parent_assignment_with_model_only_under_later_heading() -> TestResult {
    assert_rejected(
        "- Parent-to-generic-child delivery MUST pass `thinking: \"high\"`.\n\n### Historical context\nOlder work used `model: \"gpt-5.6-terra\"`.",
        "gpt-5.6-terra/high",
    )
}

#[test]
fn validator_rejects_numbered_child_assignment_with_negated_sol_model() -> TestResult {
    assert_rejected(
        "1. child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`; MUST NOT pass `model: \"gpt-5.6-sol\"`.",
        "gpt-5.6-sol/high",
    )
}

#[test]
fn validator_accepts_negated_reporting_prose_with_delivery_marker() -> TestResult {
    assert_accepted(duplicate_recipient_section(
        "- Historical prose quotes child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.",
    )?)
}

#[test]
fn validator_rejects_conflicting_child_models_in_every_active_form() -> TestResult {
    for policy in [
        "- child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"`, `model: \"gpt-5.6-sol\"`, and `thinking: \"high\"`.",
        "1. child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"`, `model: \"gpt-5.6-sol\"`, and `thinking: \"high\"`.",
        "child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"`, `model: \"gpt-5.6-sol\"`, and `thinking: \"high\"`.",
    ] {
        assert_rejected(policy, "gpt-5.6-sol/high")?;
    }
    Ok(())
}

#[test]
fn validator_rejects_conflicting_parent_model_and_effort() -> TestResult {
    assert_rejected(
        "1. Parent-to-generic-child delivery MUST pass `model: \"gpt-5.6-sol\"`, `model: \"gpt-5.6-terra\"`, `thinking: \"low\"`, and `thinking: \"high\"`.",
        "gpt-5.6-terra/high",
    )
}

#[test]
fn validator_rejects_conflicting_child_effort() -> TestResult {
    assert_rejected(
        "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"`, `thinking: \"low\"`, and `thinking: \"high\"`.",
        "gpt-5.6-sol/high",
    )
}

#[test]
fn validator_accepts_correct_fields_after_unrelated_prohibition() -> TestResult {
    assert_accepted(duplicate_recipient_section(
        "child-to-root delivery MUST pass the recipient route; MUST NOT derive it from the sender; explicitly pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
    )?)
}

#[test]
fn validator_rejects_prefixed_field_decoys_in_every_active_child_form() -> TestResult {
    for policy in [
        "- child-to-root delivery MUST pass `recipient_model: \"gpt-5.6-sol\"` and `configured_thinking: \"high\"`.",
        "1. child-to-root delivery MUST pass `recipient_model: \"gpt-5.6-sol\"` and `configured_thinking: \"high\"`.",
        "child-to-root delivery MUST pass `recipient_model: \"gpt-5.6-sol\"` and `configured_thinking: \"high\"`.",
    ] {
        assert_rejected(policy, "gpt-5.6-sol/high")?;
    }
    Ok(())
}

#[test]
fn validator_rejects_each_required_field_replaced_by_a_prefixed_decoy() -> TestResult {
    assert_rejected(
        "Parent-to-generic-child delivery MUST pass `recipient_model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.",
        "gpt-5.6-terra/high",
    )?;
    assert_rejected(
        "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `configured_thinking: \"high\"`.",
        "gpt-5.6-sol/high",
    )
}

#[test]
fn validator_rejects_hyphenated_and_dotted_field_decoys() -> TestResult {
    for policy in [
        "child-to-root delivery MUST pass `recipient-model: \"gpt-5.6-sol\"` and `configured-thinking: \"high\"`.",
        "child-to-root delivery MUST pass `recipient.model: \"gpt-5.6-sol\"` and `configured.thinking: \"high\"`.",
    ] {
        assert_rejected(policy, "gpt-5.6-sol/high")?;
    }
    Ok(())
}

#[test]
fn validator_rejects_standalone_negated_delivery_assignment() -> TestResult {
    assert_rejected(
        "child-to-root delivery MUST NOT pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
        "gpt-5.6-sol/high",
    )
}

#[test]
fn validator_accepts_correct_fields_after_period_separated_prohibition() -> TestResult {
    assert_accepted(duplicate_recipient_section(
        "child-to-root delivery MUST pass the recipient route. MUST NOT derive it from the sender. Explicitly pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
    )?)
}

#[test]
fn validator_rejects_prohibited_child_fields_after_dotted_tokens() -> TestResult {
    assert_rejected(
        "child-to-root delivery MUST pass the recipient route. MUST NOT pass `sender.model` or `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
        "gpt-5.6-sol/high",
    )
}

#[test]
fn validator_rejects_prohibited_parent_fields_after_dotted_tokens() -> TestResult {
    assert_rejected(
        "Parent-to-generic-child delivery MUST pass the recipient route. MUST NOT pass `sender.model` or `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.",
        "gpt-5.6-terra/high",
    )
}

#[test]
fn validator_rejects_prohibited_fields_after_abbreviations() -> TestResult {
    for policy in [
        "child-to-root delivery MUST pass the recipient route. MUST NOT pass sender fields, e.g. `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
        "child-to-root delivery MUST pass the recipient route. MUST NOT pass sender fields, e.g. Model route `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
    ] {
        assert_rejected(policy, "gpt-5.6-sol/high")?;
    }
    assert_rejected(
        "Parent-to-generic-child delivery MUST pass <!-- `model: \"gpt-5.6-terra\"` and\n`thinking: \"high\"` -->.",
        "gpt-5.6-terra/high",
    )
}

#[test]
fn validator_rejects_required_clause_hidden_in_inline_comment() -> TestResult {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?
    .replacen(
        "explicitly pass the recipient's configured UI `model` and `thinking`. MUST NOT",
        "<!-- explicitly pass the recipient's configured UI `model` and `thinking`. --> MUST NOT",
        1,
    );
    assert_policy_rejected(skill, "thread messages must explicitly pass")
}

#[test]
fn validator_rejects_later_incomplete_active_message_instruction() -> TestResult {
    for policy in [
        "- Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model`.",
        "1. Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model`.",
        "Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model`.",
        "  1. Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model`.",
        "   Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model`.",
    ] {
        assert_policy_rejected(
            duplicate_recipient_section(policy)?,
            "thread messages must explicitly pass",
        )?;
    }
    Ok(())
}

#[test]
fn validator_preserves_wrapped_instructions_and_chained_comments() -> TestResult {
    assert_accepted(duplicate_recipient_section(
        "1. Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model`\n   and `thinking`. MUST NOT infer either from historical actual `turn_context` state, the sender, or ambient defaults.",
    )?)?;
    assert_accepted(duplicate_recipient_section(
        "Active <!-- closed --> <!--\n1. Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model`.\n--> prose.",
    )?)?;
    assert_accepted(duplicate_recipient_section(
        "\u{2003}1. Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST explicitly pass the recipient's configured UI `model`.",
    )?)
}
