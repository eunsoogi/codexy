use std::process::Command;

mod support;

use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_appended_luna_and_sentinel_contradictions() -> TestResult {
    for (addition, expected) in [
        (
            "- `gpt-5.6-luna` MUST be the blanket default for implementation.\n",
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            "- `gpt-5.6-luna` MUST always be the blanket default for implementation.\n",
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            "- `codexy-sentinel` MUST use Ultra.\n",
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
        (
            "- `codexy-sentinel` MUST remain Ultra.\n",
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
        (
            "- `codexy-sentinel` MUST use Ultra and report the effective model.\n",
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
        (
            "- `codexy-sentinel` MUST use `Ultra`.\n",
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
        (
            "- `codexy-sentinel` MUST use Ultra for forbidden tasks.\n",
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
        (
            "- `codexy-sentinel` MUST report the effective model; MUST use Ultra.\n",
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
    ] {
        assert_rejected(appended_policy(addition)?, expected)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pass_and_passing_specialist_overrides() -> TestResult {
    for addition in [
        "- A named custom specialist MUST pass model and reasoning-effort overrides.\n",
        "- A named custom specialist MUST pass model and reasoning-effort overrides without delay.\n",
        "- A named custom specialist MUST pass model and reasoning-effort overrides without model changes.\n",
        "- A named custom specialist MUST pass model and reasoning-effort overrides without delay and report the overrides.\n",
        "- A named custom specialist MUST allow passing model and reasoning-effort overrides.\n",
    ] {
        assert_rejected(
            appended_policy(addition)?,
            "named custom specialists must keep their TOML model and reasoning effort",
        )?;
    }
    Ok(())
}

#[test]
fn validator_allows_negated_or_reporting_policy_text() -> TestResult {
    for addition in [
        "- `gpt-5.6-luna` MUST document why it is not the blanket default.\n",
        "- `gpt-5.6-luna` MUST state it will not be the blanket default.\n",
        "- `gpt-5.6-luna` MUST report that it is not the blanket default.\n",
        "- `codexy-sentinel` MUST document that Ultra is historical.\n",
        "- `codexy-sentinel` MUST report that Ultra was previously assigned.\n",
        "- `codexy-sentinel` MUST document the historical rule \"agents MUST use Ultra\".\n",
        "- A named custom specialist MUST pass validation without model or reasoning-effort overrides.\n",
    ] {
        assert_accepted(appended_policy(addition)?)?;
    }
    Ok(())
}

fn appended_policy(addition: &str) -> TestResult<String> {
    Ok(routing_skill()?.replacen("## Read Next", &format!("{addition}\n## Read Next"), 1))
}

fn routing_skill() -> TestResult<String> {
    Ok(std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?)
}

fn assert_rejected(skill: String, expected: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    std::fs::write(
        plugin_root.join("skills/codex-orchestration/SKILL.md"),
        &skill,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "routing bypass unexpectedly passed:\n{skill}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_accepted(skill: String) -> TestResult {
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

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "valid routing policy rejected:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
