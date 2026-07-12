use std::process::Command;

mod support;

use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn orchestration_skill_declares_the_gpt_5_6_routing_matrix() -> TestResult {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?;

    for required in [
        "## GPT-5.6 Routing Matrix",
        "Root/orchestrator",
        "`gpt-5.6-sol`",
        "Generic implementation, debugging, integration, and QA child thread",
        "model: \"gpt-5.6-terra\"",
        "`gpt-5.6-luna`",
        "repository discovery, cataloging, simple",
        "documentation drafting, bounded polling, and repetitive checks",
        "Cost guidance: Luna is an optimization for bounded low-risk work",
        "quality-neutral replacement for Terra",
        "MUST NOT use",
        "Luna as the blanket default",
        "named custom specialist",
        "TOML is the model and reasoning-effort source of",
        "truth. MUST NOT pass model or reasoning-effort overrides",
        "`codexy-sentinel` remains `gpt-5.6-sol` / `xhigh`. MUST NOT use Ultra",
        "MUST NOT pass model or reasoning-effort overrides",
        "fork_turns=\"none\"",
    ] {
        assert!(
            skill.contains(required),
            "missing routing contract: {required}"
        );
    }
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
        assert!(skill.contains(needle), "test fixture is missing {needle:?}");
        std::fs::write(&path, skill.replacen(needle, replacement, 1))?;

        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args([
                "--plugin-root",
                plugin_root.to_str().ok_or("plugin root")?,
                "--check",
            ])
            .output()?;
        assert!(
            !output.status.success(),
            "regression {replacement:?} unexpectedly passed"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
