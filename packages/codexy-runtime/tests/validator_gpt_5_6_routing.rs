use std::path::{Path, PathBuf};
use std::process::Command;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn assert_rejected_routing_skill(skill: String, expected: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = routing_fixture(&temp)?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let source = std::fs::read_to_string(source_routing_skill())?;
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
    assert_eq!(
        std::fs::read_to_string(source_routing_skill())?,
        source,
        "routing fixture mutation escaped its declared copy-on-write file"
    );
    Ok(())
}

fn routing_fixture(temp: &tempfile::TempDir) -> std::io::Result<PathBuf> {
    let plugin_root = temp.path().join("codexy");
    support::copy_plugin_fixture_into_with_mutable_files(
        &plugin_root,
        &[Path::new("skills/codex-orchestration/SKILL.md")],
    )?;
    Ok(plugin_root)
}

fn source_routing_skill() -> PathBuf {
    codexy_runtime::paths::repository_root()
        .join("plugins/codexy/skills/codex-orchestration/SKILL.md")
}

#[test]
fn orchestration_skill_declares_the_gpt_5_6_routing_matrix() -> TestResult {
    let plugin_root = codexy_runtime::paths::repository_root().join("plugins/codexy");
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
    let skill = std::fs::read_to_string(source_routing_skill())?;
    let (needle, replacement, expected) = (
        "`gpt-5.6-sol` for decomposition",
        "`gpt-5.6-terra` for decomposition",
        "root/orchestrator must use gpt-5.6-sol",
    );
    let mutated = skill.replacen(needle, replacement, 1);
    assert_ne!(skill, mutated, "test fixture is missing {needle:?}");
    assert_rejected_routing_skill(mutated, expected)?;
    Ok(())
}
