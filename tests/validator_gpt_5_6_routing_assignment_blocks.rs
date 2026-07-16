use std::process::Command;

mod support;

use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
    let output = validate(duplicate_recipient_section(
        "- Historical prose quotes child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.",
    )?)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
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
    let output = validate(duplicate_recipient_section(
        "child-to-root delivery MUST pass the recipient route; MUST NOT derive it from the sender; explicitly pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
    )?)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
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
    let output = validate(duplicate_recipient_section(
        "child-to-root delivery MUST pass the recipient route. MUST NOT derive it from the sender. Explicitly pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
    )?)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_rejected(policy: &str, expected: &str) -> TestResult {
    let output = validate(duplicate_recipient_section(policy)?)?;
    assert!(
        !output.status.success(),
        "routing bypass unexpectedly passed"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "routing rejection must name {expected}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn duplicate_recipient_section(policy: &str) -> TestResult<String> {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?;
    Ok(skill.replacen(
        "## Read Next",
        &format!("## Recipient Model Routing\n\n{policy}\n\n## Read Next"),
        1,
    ))
}

fn validate(skill: String) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    std::fs::write(
        plugin_root.join("skills/codex-orchestration/SKILL.md"),
        skill,
    )?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?)
}
