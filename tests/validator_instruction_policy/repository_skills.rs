use std::path::{Path, PathBuf};

use crate::support;

use super::{TestResult, copy_plugin_fixture, stderr, validator};

#[test]
fn validator_cli_rejects_repository_skill_instruction_policy_regressions() -> TestResult {
    let (_temp, plugin_root, _agents_path) = copy_repo_fixture()?;
    let skill_path = plugin_root
        .parent()
        .and_then(Path::parent)
        .ok_or("repository root missing")?
        .join(".agents/skills/release-engineering/SKILL.md");
    let prompt_path = skill_path
        .parent()
        .ok_or("repository skill directory missing")?
        .join("agents/openai.yaml");
    let skill = std::fs::read_to_string(&skill_path)?;
    let prompt = std::fs::read_to_string(&prompt_path)?;

    std::fs::write(&skill_path, skill.replace("MUST keep", "keep"))?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));

    std::fs::write(&skill_path, skill)?;
    std::fs::write(
        &prompt_path,
        prompt.replace("You MUST run $task-classification", "Run $task-classification"),
    )?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

#[test]
fn installed_plugin_validation_ignores_adjacent_repository_skill_roots() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let project_skill = plugin_root
        .parent()
        .ok_or("plugin parent missing")?
        .join(".agents/skills/release-engineering/agents/openai.yaml");
    std::fs::create_dir_all(project_skill.parent().ok_or("project skill parent missing")?)?;
    std::fs::write(&project_skill, "not: valid: yaml\n")?;

    let output = validator(&plugin_root, "--check")?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

pub(crate) fn copy_repo_fixture() -> TestResult<(tempfile::TempDir, PathBuf, PathBuf)> {
    let temp = tempfile::tempdir()?;
    let repo_root = temp.path().join("repo");
    let plugin_root = repo_root.join("plugins/codexy");
    let agents_path = repo_root.join("AGENTS.md");
    std::fs::create_dir_all(repo_root.join("plugins"))?;
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md"),
        &agents_path,
    )?;
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".agents/skills"),
        &repo_root.join(".agents/skills"),
    )?;
    Ok((temp, plugin_root, agents_path))
}
