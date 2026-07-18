use crate::support;

use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_contradictory_routing_and_specialist_override_bypasses() -> TestResult {
    for (addition, expected) in [
        (
            "- Root/orchestrator: MUST use `gpt-5.6-luna`.\n",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- Generic implementation child MUST use `gpt-5.6-luna`.\n",
            "generic child thread must explicitly request gpt-5.6-terra/high",
        ),
        (
            "- A named custom specialist MUST set model and reasoning_effort at spawn; MUST NOT modify source files.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- A named custom specialist MUST set model and reasoning_effort at spawn.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- Root/orchestrator: MUST use `gpt-5.6-sol` or `gpt-5.6-luna`.\n",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- Generic implementation child MUST use `gpt-5.6-sol`.\n",
            "generic child thread must explicitly request gpt-5.6-terra/high",
        ),
        (
            "- A named custom specialist MUST use model: `gpt-5.6-luna` and reasoning_effort: `high`.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- Root/orchestrator: MUST also use `gpt-5.6-terra`.\n",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- Generic implementation child MUST use `gpt-5.5`.\n",
            "generic child thread must explicitly request gpt-5.6-terra/high",
        ),
        (
            "- A named custom specialist MUST set model: `gpt-5.6-luna`.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- Root/orchestrator: MUST use `o4-mini`.\n",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- Root/orchestrator: MUST run on `o4-mini`.\n",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- Root/orchestrator: MUST run on `o3`.\n",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- Root/orchestrator: MUST run with `o3`.\n",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- Generic implementation child MUST select `gpt-5.6-sol`.\n",
            "generic child thread must explicitly request gpt-5.6-terra/high",
        ),
        (
            "- A named custom specialist MUST preserve its TOML model unchanged, but MUST set reasoning_effort at spawn.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- A named custom specialist MUST preserve its TOML model unchanged and set reasoning_effort to high.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- A named custom specialist MUST run on `gpt-5.6-luna`.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- A named custom specialist MUST run using `gpt-5.6-luna`.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- A named custom specialist MUST choose model `gpt-5.6-luna`.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- A named custom specialist MUST choose model `gpt-5.6-luna` without modifying source files.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- A named custom specialist MUST preserve its TOML unchanged and choose model `gpt-5.6-luna`.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- Root/orchestrator MUST use `gpt-5.6-terra` for comparison only; it is not the assigned model, and run on `o3` for fallback.\n",
            "root/orchestrator must use gpt-5.6-sol",
        ),
        (
            "- A named custom specialist MUST set model and reasoning_effort at spawn without modifying source files.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            "- A named custom specialist MUST report its model and set reasoning_effort at spawn.\n",
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
    ] {
        assert_rejected(
            routing_skill()?.replacen("## Read Next", &format!("{addition}\n## Read Next"), 1),
            expected,
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_commented_or_appended_override_policy() -> TestResult {
    assert_rejected(
        format!("<!--\n{}\n-->", routing_skill()?),
        "must define the GPT-5.6 routing matrix",
    )?;
    assert_rejected(
        routing_skill()?.replacen(
            "truth. MUST NOT pass model or reasoning-effort overrides.",
            "truth. MUST NOT pass model or reasoning-effort overrides. However, it MUST set model and reasoning_effort at spawn.",
            1,
        ),
        "named custom specialists must keep their TOML model and reasoning effort",
    )
}

#[test]
fn validator_allows_preservation_wording_and_fenced_comment_literals() -> TestResult {
    assert_accepted(routing_skill()?.replacen(
        "## Read Next",
        "- A named custom specialist MUST preserve its TOML model and reasoning-effort settings unchanged.\n\n## Read Next",
        1,
    ))?;
    assert_accepted(routing_skill()?.replacen(
        "## Read Next",
        "- Root/orchestrator MUST use `gpt-5.6-terra` for comparison only; it is not the assigned model.\n\n## Read Next",
        1,
    ))?;
    assert_accepted(routing_skill()?.replacen(
        "## Read Next",
        "- Root/orchestrator MUST use `gpt-5.6-terra` only as a documented comparison, not as its assigned model.\n\n## Read Next",
        1,
    ))?;
    assert_accepted(routing_skill()?.replacen(
        "## Read Next",
        "- A named custom specialist MUST report its effective model and reasoning_effort.\n- A named custom specialist MUST spawn without model or reasoning-effort overrides.\n- Root/orchestrator MUST document why `gpt-5.6-terra` is not selected.\n\n## Read Next",
        1,
    ))?;
    assert_accepted(routing_skill()?.replacen(
        "## Read Next",
        "- Root/orchestrator MUST use explicit risk controls.\n- Root/orchestrator MUST review model-output evidence.\n- A named custom specialist MUST treat its TOML as the source of truth for model and reasoning-effort.\n\n## Read Next",
        1,
    ))?;
    assert_accepted(routing_skill()?.replacen(
        "## Read Next",
        "- Root/orchestrator MUST NOT use `gpt-5.6-terra`.\n- Generic implementation child MUST NOT use `gpt-5.6-sol`.\n- A named custom specialist MUST keep its TOML model and reasoning-effort settings unchanged.\n\n## Read Next",
        1,
    ))?;
    assert_accepted(format!(
        "```text\n<!-- historical literal without an HTML close\n```\n\n{}",
        routing_skill()?
    ))
}

#[test]
fn validator_rejects_an_indented_historical_routing_matrix() -> TestResult {
    let indented = routing_skill()?
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_rejected(indented, "must define the GPT-5.6 routing matrix")
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
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    std::fs::write(path, &skill)?;

    let output = support::validator_routing(&plugin_root)?;
    assert!(
        !output.status.success(),
        "routing bypass unexpectedly passed:\n{skill}"
    );
    support::assert_structured_literals(
        &String::from_utf8_lossy(&output.stderr),
        "routing rejection diagnostic",
        &[expected],
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
    let output = support::validator_routing(&plugin_root)?;
    assert!(
        output.status.success(),
        "valid routing policy rejected:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
