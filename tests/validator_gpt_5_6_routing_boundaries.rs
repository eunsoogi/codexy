use std::process::Command;

#[path = "structured_contract.rs"]
mod structured_contract;
mod support;

use structured_contract::{Contract, Modality, Rule};
use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_conflicts_appended_to_delivery_policy() -> TestResult {
    assert_rejected(
        routing_skill()?.replacen(
            "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"`\n  and `thinking: \"high\"`.",
            "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"`\n  and `thinking: \"high\"`. Root/orchestrator MUST use `gpt-5.6-luna`.",
            1,
        ),
    )
}

#[test]
fn validator_rejects_duplicate_recipient_heading_that_masks_conflict() -> TestResult {
    let skill = routing_skill()?;
    let start = skill
        .find("## Recipient Model Routing")
        .ok_or("recipient heading")?;
    let end = skill.find("## Read Next").ok_or("read-next heading")?;
    let duplicate = &skill[start..end];
    let mutated = skill
        .replacen(
            "## Recipient Model Routing",
            "## Recipient Model Routing\n\n- Root/orchestrator: MUST use `gpt-5.6-luna`.",
            1,
        )
        .replacen("## Read Next", &format!("{duplicate}## Read Next"), 1);
    assert_rejected(mutated)
}

#[test]
fn validator_rejects_duplicate_recipient_heading_after_valid_section() -> TestResult {
    let skill = routing_skill()?;
    let start = skill
        .find("## Recipient Model Routing")
        .ok_or("recipient routing heading")?;
    let end = skill.find("## Read Next").ok_or("read next heading")?;
    let duplicate = skill[start..end].replacen(
        "## Recipient Model Routing",
        "## Recipient Model Routing\n\n- Root/orchestrator: MUST use `gpt-5.6-luna`.",
        1,
    );
    let mutated = skill.replacen("## Read Next", &format!("{duplicate}## Read Next"), 1);

    assert_rejected(mutated)
}

#[test]
fn validator_rejects_later_child_to_root_wrong_recipient_model() -> TestResult {
    assert_recipient_assignment_rejected(duplicate_recipient_section(
        routing_skill()?,
        "child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"`\n  and `thinking: \"high\"`.",
    )?)
}

#[test]
fn validator_rejects_later_child_to_root_without_thinking() -> TestResult {
    assert_recipient_assignment_rejected(duplicate_recipient_section(
        routing_skill()?,
        "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"`.",
    )?)
}

#[test]
fn validator_rejects_active_policy_after_closed_html_comment() -> TestResult {
    assert_rejected(routing_skill()?.replacen(
        "## Read Next",
        "<!-- historical note --> - Root/orchestrator: MUST use `gpt-5.6-luna`.\n\n## Read Next",
        1,
    ))
}

#[test]
fn validator_accepts_heading_and_paragraph_after_exact_evidence() -> TestResult {
    assert_accepted(routing_skill()?.replacen(
        "## Read Next",
        "### Context\nThis explanatory paragraph is not routing evidence.\n\n## Read Next",
        1,
    ))
}

fn routing_skill() -> TestResult<String> {
    Ok(std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?)
}

fn assert_rejected(skill: String) -> TestResult {
    Contract::markdown(&skill)
        .assert_rule(
            Rule::new(
                "routing.fixture.forbidden-luna-recipient",
                "root/orchestrator",
                Modality::Required,
                &["use"],
                &[],
            )
            .under_heading("recipient model routing"),
        )
        .expect("routing fixture must contain the forbidden recipient policy");
    let output = validate(skill)?;
    assert!(
        !output.status.success(),
        "routing bypass unexpectedly passed"
    );
    Ok(())
}

fn assert_recipient_assignment_rejected(skill: String) -> TestResult {
    let output = validate(skill)?;
    assert!(
        !output.status.success(),
        "routing bypass unexpectedly passed"
    );
    Ok(())
}

fn duplicate_recipient_section(skill: String, child_to_root: &str) -> TestResult<String> {
    assert!(
        skill.find("## Recipient Model Routing").is_some(),
        "recipient routing heading missing"
    );
    assert!(
        skill.find("## Read Next").is_some(),
        "read next heading missing"
    );
    let duplicate = format!("## Recipient Model Routing\n\n- {child_to_root}\n\n");
    Ok(skill.replacen("## Read Next", &format!("{duplicate}## Read Next"), 1))
}

fn assert_accepted(skill: String) -> TestResult {
    let output = validate(skill)?;
    assert!(
        output.status.success(),
        "valid routing policy rejected:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
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
