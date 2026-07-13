use std::process::Command;

mod support;

use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_contextual_gpt_5_6_routing_bypasses() -> TestResult {
    for (needle, replacement, expected) in [
        (
            "## GPT-5.6 Routing Matrix",
            "## Historical Non-Policy Example\n\n```text\n## GPT-5.6 Routing Matrix",
            "must define the GPT-5.6 routing matrix",
        ),
        (
            "## GPT-5.6 Routing Matrix",
            "## Historical Non-Policy Example\n\n````text\n## GPT-5.6 Routing Matrix",
            "must define the GPT-5.6 routing matrix",
        ),
    ] {
        let close_fence = if replacement.contains("````") {
            "````\n\n## Read Next"
        } else {
            "```\n\n## Read Next"
        };
        assert_routing_rejected(
            |skill| {
                skill
                    .replacen(needle, replacement, 1)
                    .replacen("## Read Next", close_fence, 1)
            },
            expected,
        )?;
    }
    for (replacement, close_fence) in [
        (
            "## Historical Non-Policy Example\n\n```text\n## GPT-5.6 Routing Matrix",
            "```not-a-closing-fence\n## Read Next",
        ),
        (
            "## Historical Non-Policy Example\n\n```text\n~~~\n## GPT-5.6 Routing Matrix",
            "```\n\n## Read Next",
        ),
    ] {
        assert_routing_rejected(
            |skill| {
                skill
                    .replacen("## GPT-5.6 Routing Matrix", replacement, 1)
                    .replacen("## Read Next", close_fence, 1)
            },
            "must define the GPT-5.6 routing matrix",
        )?;
    }
    for (needle, replacement, expected) in [
        (
            "- Root/orchestrator: MUST use `gpt-5.6-sol`",
            "- MUST NOT follow this obsolete example: Root/orchestrator: MUST use `gpt-5.6-sol`",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- Generic implementation, debugging, integration, and QA child thread: MUST",
            "- MUST NOT follow this obsolete example: Generic implementation, debugging, integration, and QA child thread: MUST",
            "generic child thread must explicitly request gpt-5.6-terra/high",
        ),
        (
            "- `gpt-5.6-luna` is only for repository discovery, cataloging, simple",
            "- MUST NOT follow this obsolete example: `gpt-5.6-luna` is only for repository discovery, cataloging, simple",
            "Luna must stay limited to enumerated low-risk mechanical work",
        ),
    ] {
        assert_routing_rejected(|skill| skill.replacen(needle, replacement, 1), expected)?;
    }
    assert_routing_rejected(
        |skill| {
            skill.replacen(
                "Generic implementation, debugging, integration, and QA child thread: MUST\n  explicitly request",
                "Generic implementation, debugging, integration, and QA child thread: MUST NOT\n  explicitly request",
                1,
            )
        },
        "generic child thread must explicitly request gpt-5.6-terra/high",
    )?;
    for replacement in [
        "Named custom specialists MUST receive model and reasoning-effort overrides at spawn time.",
        "A named custom specialist MUST receive model and reasoning_effort overrides at spawn time.",
        "Named custom specialists MUST be spawned with model and reasoning-effort overrides.",
    ] {
        assert_routing_rejected(
            |skill| {
                skill.replacen(
                    "truth. MUST NOT pass model or reasoning-effort overrides.",
                    &format!(
                        "truth. MUST NOT pass model or reasoning-effort overrides.\n- {replacement}"
                    ),
                    1,
                )
            },
            "named custom specialists must keep their TOML model and reasoning effort",
        )?;
    }
    for (replacement, expected) in [
        (
            "- `gpt-5.6-luna` MUST use Luna as the blanket default for implementation.\n\n## Read Next",
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            "- `gpt-5.6-luna` MUST use Luna as the blanket default, not Terra.\n\n## Read Next",
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            "- `gpt-5.6-luna` MUST use Luna as the blanket default for implementation, but not be the blanket default for security review.\n\n## Read Next",
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            "- `gpt-5.6-luna` MUST report that Luna is available for discovery, but use Luna as the blanket default for implementation.\n\n## Read Next",
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            "- `gpt-5.6-luna` MUST report that Luna is available for comparison, not as the blanket default but make Luna the blanket default for implementation.\n\n## Read Next",
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            "- `gpt-5.6-luna` MUST report that Luna is available for comparison, not as the blanket default and make Luna the blanket default for implementation.\n\n## Read Next",
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            "- `codexy-sentinel` MUST run on `gpt-5.6-terra` with high reasoning.\n\n## Read Next",
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
    ] {
        assert_routing_rejected(
            |skill| skill.replacen("## Read Next", replacement, 1),
            expected,
        )?;
    }
    assert_routing_rejected(
        |skill| {
            skill.replacen(
                "## GPT-5.6 Routing Matrix",
                "## GPT-5.6 Routing Matrix\n\n## Historical Non-Policy Example",
                1,
            )
        },
        "generic child thread must explicitly request gpt-5.6-terra/high",
    )
}

#[test]
fn validator_cli_accepts_reporting_about_luna_default_policy() -> TestResult {
    assert_routing_accepted(|skill| {
        skill.replacen(
            "## Read Next",
            "- `gpt-5.6-luna` MUST report that Luna may be used for comparison, not as the blanket default.\n\n## Read Next",
            1,
        )
    })
}

#[test]
fn validator_rejects_normalized_luna_blanket_default_assignments() -> TestResult {
    for replacement in [
        "- `GPT-5.6-LUNA` MUST ASSIGN Luna as the blanket-default for implementation.\n\n## Read Next",
        "- `gpt-5.6-luna` MUST report comparison availability, BUT MUST assign Luna as\tthe blanket default for implementation.\n\n## Read Next",
    ] {
        assert_routing_rejected(
            |skill| skill.replacen("## Read Next", replacement, 1),
            "Luna must remain limited to bounded mechanical work",
        )?;
    }
    Ok(())
}

fn assert_routing_rejected(mutate: impl FnOnce(String) -> String, expected: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    std::fs::write(&path, mutate(std::fs::read_to_string(&path)?))?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "routing bypass unexpectedly passed"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_routing_accepted(mutate: impl FnOnce(String) -> String) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    std::fs::write(&path, mutate(std::fs::read_to_string(&path)?))?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "reporting prose unexpectedly failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
