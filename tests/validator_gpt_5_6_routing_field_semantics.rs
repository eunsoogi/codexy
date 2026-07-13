use std::process::Command;

mod support;

use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn rejects_effort_only_specialist_assignments() -> TestResult {
    for addition in [
        "- A named custom specialist MUST choose reasoning_effort high.",
        "- A named custom specialist MUST use reasoning-effort high.",
        "- A named custom specialist MUST select reasoning_effort high without changing its model.",
        "- A named custom specialist MUST use its TOML model unchanged and choose reasoning_effort high.",
        "- A named custom specialist MUST keep its TOML model unchanged and select reasoning-effort high.",
        "- A named custom specialist MUST keep its TOML model unchanged and use reasoning_effort high.",
        "- A named custom specialist MUST keep its TOML model and reasoning_effort unchanged and choose reasoning_effort high.",
        "- A named custom specialist MUST use its TOML model and reasoning_effort unchanged and choose model default.",
        "- A named custom specialist MUST choose reasoning_effort high and keep its TOML model and reasoning_effort unchanged.",
        "- A named custom specialist MUST choose model default and keep its TOML model and reasoning_effort unchanged.",
        "- A named custom specialist MUST choose reasoning_effort low and keep its TOML model and reasoning_effort unchanged.",
        "- A named custom specialist MUST select reasoning-effort xhigh and keep its TOML model and reasoning-effort unchanged.",
        "- A named custom specialist MUST choose reasoning_effort low and choose the model and reasoning_effort declared by its TOML unchanged.",
        "- A named custom specialist MUST choose model default and choose the model and reasoning_effort declared by its TOML unchanged.",
        "- A named custom specialist MUST choose reasoning_effort low and the model and reasoning_effort declared by its TOML unchanged.",
        "- A named custom specialist MUST choose model default and the model and reasoning_effort declared by its TOML unchanged.",
    ] {
        assert_status(addition, false)?;
    }
    Ok(())
}

#[test]
fn rejects_generic_child_effort_downgrades() -> TestResult {
    for addition in [
        "- Generic implementation child MUST request model: \"gpt-5.6-terra\" and reasoning_effort: \"low\".",
        "- Generic QA child MUST request model: \"gpt-5.6-terra\" and reasoning-effort: xhigh.",
        "- Generic implementation child MUST set reasoning_effort to low.",
        "- Generic QA child MUST set reasoning-effort to xhigh.",
        "- Generic implementation child MUST use Ultra.",
        "- Root/orchestrator MUST run using the Ultra model.",
    ] {
        assert_status(addition, false)?;
    }
    Ok(())
}

#[test]
fn rejects_sentinel_effort_downgrades() -> TestResult {
    for addition in [
        "- `codexy-sentinel` MUST use `gpt-5.6-sol` with `reasoning_effort: high`.",
        "- `codexy-sentinel` MUST set reasoning-effort to low.",
        "- `codexy-sentinel` MUST use the Ultra model.",
    ] {
        assert_status(addition, false)?;
    }
    Ok(())
}

#[test]
fn rejects_singular_specialist_overrides() -> TestResult {
    for addition in [
        "- A named custom specialist MUST pass a model override.",
        "- A named custom specialist MUST pass a reasoning-effort override.",
    ] {
        assert_status(addition, false)?;
    }
    Ok(())
}

#[test]
fn allows_repeated_comparison_model_mentions() -> TestResult {
    assert_status(
        "- Root/orchestrator MUST use `gpt-5.6-terra` for comparison only; `gpt-5.6-terra` is not the assigned model.",
        true,
    )
}

#[test]
fn allows_selection_of_unchanged_toml_values() -> TestResult {
    for addition in [
        "- A named custom specialist MUST use its TOML model and reasoning_effort unchanged.",
        "- A named custom specialist MUST keep and use its TOML model and reasoning_effort unchanged.",
        "- A named custom specialist MUST choose the model and reasoning_effort declared by its TOML unchanged.",
    ] {
        assert_status(addition, true)?;
    }
    Ok(())
}

fn assert_status(addition: &str, accepted: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&path)?.replacen(
        "## Read Next",
        &format!("{addition}\n\n## Read Next"),
        1,
    );
    std::fs::write(path, skill)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?;
    assert_eq!(
        output.status.success(),
        accepted,
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
