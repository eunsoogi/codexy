use std::process::Command;

use crate::support;

use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn assert_rejected_routing_skill(skill: String, expected: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    std::fs::write(&path, skill)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "routing regression unexpectedly passed"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn orchestration_skill_declares_the_gpt_5_6_routing_matrix() -> TestResult {
    let plugin_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy");
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?;
    assert!(output.status.success(), "routing contract rejected");
    Ok(())
}

#[test]
fn validator_cli_rejects_gpt_5_6_routing_regressions() -> TestResult {
    for (needle, replacement, expected) in [
        (
            "`gpt-5.6-sol` for decomposition",
            "`gpt-5.6-terra` for decomposition",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "model: \"gpt-5.6-terra\"",
            "model: \"gpt-5.6-luna\"",
            "generic child thread must explicitly request gpt-5.6-terra/high",
        ),
        (
            "MUST NOT pass model or reasoning-effort overrides.",
            "MUST NOT pass model overrides.",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "`codexy-sentinel` remains `gpt-5.6-sol` / `xhigh`. MUST NOT use Ultra",
            "`codexy-sentinel` remains `gpt-5.6-terra` / `ultra`. MUST NOT use Ultra",
            "codexy-sentinel must remain gpt-5.6-sol/xhigh and MUST NOT use Ultra",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
        let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&path)?;
        let mutated = skill.replacen(needle, replacement, 1);
        assert_ne!(skill, mutated, "test fixture is missing {needle:?}");
        assert_rejected_routing_skill(mutated, expected)?;
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_recipient_model_routing_regressions() -> TestResult {
    for (needle, replacement, expected) in [
        (
            "## Recipient Model Routing",
            "## Message Routing",
            "must define recipient model routing policy",
        ),
        (
            "destination owner's configured UI `model` and `thinking`",
            "destination owner's configured UI `thinking`",
            "active child/parent thread ledger must record the configured UI model and thinking",
        ),
        (
            "recipient's configured UI `model` and `thinking`",
            "recipient's configured UI `model`",
            "thread messages must explicitly pass the recipient model and thinking",
        ),
        (
            "Parent-to-generic-child delivery MUST pass `model: \"gpt-5.6-terra\"` and\n  `thinking: \"high\"`",
            "Parent-to-generic-child delivery MUST pass `model: \"gpt-5.6-sol\"` and\n  `thinking: \"high\"`",
            "parent-to-generic-child messages must use recipient gpt-5.6-terra/high",
        ),
        (
            "child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"`\n  and `thinking: \"high\"`",
            "child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"`\n  and `thinking: \"high\"`",
            "child-to-root messages must use recipient gpt-5.6-sol/high",
        ),
        (
            "MUST NOT\n  infer either from historical actual `turn_context` state, the sender, or ambient defaults.",
            "MUST\n  infer both from the sender's historical actual `turn_context` state.",
            "thread messages must explicitly pass the recipient model and thinking",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
        let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&path)?;
        let mutated = skill.replacen(needle, replacement, 1);
        assert_ne!(skill, mutated, "test fixture is missing {needle:?}");
        assert_rejected_routing_skill(mutated, expected)?;
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_actual_recipient_routing_evidence_regressions() -> TestResult {
    let fixture = r#"- Captured #433 parent-to-generic-child evidence: configured_ui_model="gpt-5.6-terra"; actual_turn_context_model="gpt-5.6-sol"; per_message_model="gpt-5.6-terra"; send_message_to_thread({ threadId: "child-433", model: "gpt-5.6-terra", thinking: "high" }).
- Reverse child-to-root evidence: configured_ui_model="gpt-5.6-sol"; actual_turn_context_model="gpt-5.6-terra"; per_message_model="gpt-5.6-sol"; send_message_to_thread({ threadId: "root-433", model: "gpt-5.6-sol", thinking: "high" }).

"#;
    for (needle, replacement, expected) in [
        (
            "model: \"gpt-5.6-terra\", thinking: \"high\"",
            "thinking: \"high\"",
            "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
        ),
        (
            "model: \"gpt-5.6-terra\", thinking: \"high\"",
            "model: \"gpt-5.6-terra\"",
            "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
        ),
        (
            "model: \"gpt-5.6-terra\", thinking: \"high\"",
            "model: \"gpt-5.6-sol\", thinking: \"high\"",
            "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
        ),
        (
            "model: \"gpt-5.6-sol\", thinking: \"high\"",
            "thinking: \"high\"",
            "child-to-root evidence must pass recipient gpt-5.6-sol/high",
        ),
        (
            "model: \"gpt-5.6-sol\", thinking: \"high\"",
            "model: \"gpt-5.6-sol\"",
            "child-to-root evidence must pass recipient gpt-5.6-sol/high",
        ),
        (
            "model: \"gpt-5.6-sol\", thinking: \"high\"",
            "model: \"gpt-5.6-terra\", thinking: \"high\"",
            "child-to-root evidence must pass recipient gpt-5.6-sol/high",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
        let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&path)?;
        let evidence = fixture.replacen(needle, replacement, 1);
        assert_ne!(fixture, evidence, "test fixture is missing {needle:?}");
        let mutated = skill.replacen("## Read Next", &format!("{evidence}## Read Next"), 1);
        assert_rejected_routing_skill(mutated, expected)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_decoy_and_inactive_recipient_evidence() -> TestResult {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?;
    for (needle, replacement, expected) in [
        (
            "Captured #433 parent-to-generic-child evidence: configured_ui_model=\"gpt-5.6-terra\"; actual_turn_context_model=\"gpt-5.6-sol\"; per_message_model=\"gpt-5.6-terra\"; send_message_to_thread({ threadId: \"child-433\", model: \"gpt-5.6-terra\", thinking: \"high\" })",
            "Captured #433 parent-to-generic-child evidence: not_configured_ui_model=\"gpt-5.6-terra\"; not_actual_turn_context_model=\"gpt-5.6-sol\"; not_per_message_model=\"gpt-5.6-terra\"; send_message_to_thread({ threadId: \"child-433\", model: \"gpt-5.6-sol\", recipient_model: \"gpt-5.6-terra\", model: \"gpt-5.6-terra\", thinking: \"low\", configured_thinking: \"high\", thinking: \"high\" })",
            "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
        ),
        (
            "Reverse child-to-root evidence: configured_ui_model=\"gpt-5.6-sol\"; actual_turn_context_model=\"gpt-5.6-terra\"; per_message_model=\"gpt-5.6-sol\"; send_message_to_thread({ threadId: \"root-433\", model: \"gpt-5.6-sol\", thinking: \"high\" })",
            "Reverse child-to-root evidence: not_configured_ui_model=\"gpt-5.6-sol\"; not_actual_turn_context_model=\"gpt-5.6-terra\"; not_per_message_model=\"gpt-5.6-sol\"; send_message_to_thread({ threadId: \"root-433\", model: \"gpt-5.6-terra\", recipient_model: \"gpt-5.6-sol\", model: \"gpt-5.6-sol\", thinking: \"low\", configured_thinking: \"high\", thinking: \"high\" })",
            "child-to-root evidence must pass recipient gpt-5.6-sol/high",
        ),
        (
            "- Captured #433 parent-to-generic-child evidence:",
            "<!-- - Captured #433 parent-to-generic-child evidence:",
            "parent-to-generic-child evidence must pass recipient gpt-5.6-terra/high",
        ),
        (
            "- Reverse child-to-root evidence:",
            "```text\n- Reverse child-to-root evidence:",
            "child-to-root evidence must pass recipient gpt-5.6-sol/high",
        ),
    ] {
        let mutated = skill.replacen(needle, replacement, 1);
        assert_ne!(skill, mutated, "test fixture is missing {needle:?}");
        assert_rejected_routing_skill(mutated, expected)?;
    }
    Ok(())
}
