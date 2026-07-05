use std::path::{Path, PathBuf};
use std::process::Command;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_missing_child_thread_ledger_contract() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    assert!(skill.contains("Active child Codex app threads MUST be capped"));
    assert!(skill.contains("blocker, latest evidence, and next action"));
    std::fs::write(
        &skill_path,
        skill
            .replace(
                "Active child Codex app threads MUST be capped",
                "Active child Codex app threads have a bounded concurrency limit",
            )
            .replace(
                "blocker, latest evidence, and next action",
                "blocker, and next action",
            ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("active child codex app threads must be capped at 5"));
    assert!(stderr.contains("latest evidence"));
    Ok(())
}

fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok((temp, plugin_root))
}

fn validator(
    plugin_root: &Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let root = plugin_root.to_str().ok_or("plugin root path")?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root, mode])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
