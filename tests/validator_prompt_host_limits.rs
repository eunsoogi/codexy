use std::process::Command;

use crate::support;

use support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn packaged_manifest_prompts_fit_host_limit_and_keep_primary_routing() -> TestResult {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy/.codex-plugin/plugin.json");
    let text = std::fs::read_to_string(manifest_path)?;
    let manifest: serde_json::Value = serde_json::from_str(&text)?;
    let prompts = manifest["interface"]["defaultPrompt"]
        .as_array()
        .ok_or("defaultPrompt")?;

    assert!(prompts.iter().all(|prompt| {
        prompt
            .as_str()
            .is_some_and(|prompt| prompt.chars().count() <= 128)
    }));
    support::assert_structured_literals(
        prompts[0].as_str().ok_or("primary defaultPrompt")?,
        "primary default prompt routing",
        &["$task-classification", "$codex-orchestration"],
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_manifest_prompt_over_host_limit() -> TestResult {
    let (_temp, plugin_root) = prompt_fixture(129)?;
    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains(
        "interface.defaultPrompt[1] must be at most 128 characters (found 129)"
    ));
    Ok(())
}

#[test]
fn validator_cli_rejects_too_many_manifest_prompts() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let text = std::fs::read_to_string(&manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&text)?;
    manifest["interface"]["defaultPrompt"]
        .as_array_mut()
        .ok_or("defaultPrompt")?
        .push(serde_json::json!("x"));
    std::fs::write(manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains(
        "interface.defaultPrompt must contain at most 3 entries (found 4)"
    ));
    Ok(())
}

#[test]
fn validator_cli_accepts_manifest_prompt_at_host_limit() -> TestResult {
    let (_temp, plugin_root) = prompt_fixture(128)?;
    let output = validator(&plugin_root)?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn prompt_fixture(length: usize) -> TestResult<(tempfile::TempDir, std::path::PathBuf)> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let text = std::fs::read_to_string(&manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&text)?;
    manifest["interface"]["defaultPrompt"][1] = serde_json::json!("x".repeat(length));
    std::fs::write(manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    Ok((temp, plugin_root))
}

fn copy_plugin(plugin_root: &std::path::Path) -> std::io::Result<()> {
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}

fn validator(plugin_root: &std::path::Path) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", plugin_root.to_str().ok_or("plugin root")?, "--check"])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
