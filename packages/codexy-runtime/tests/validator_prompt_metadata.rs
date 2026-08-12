use std::process::Command;

use crate::support;

use support::copy_dir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_manifest_prompt_without_orchestration_route() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let text = std::fs::read_to_string(&manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&text)?;
    assert!(
        manifest["interface"]["defaultPrompt"]
            .as_array()
            .ok_or("defaultPrompt")?
            .iter()
            .all(|line| line.as_str().is_some_and(|line| line.contains("MUST")))
    );
    manifest["interface"]["defaultPrompt"] = serde_json::json!(["Use Codexy as orchestrator."]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("interface.defaultPrompt must route through"));
    Ok(())
}

#[test]
fn validator_cli_rejects_top_level_prompt_without_orchestration_route() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let prompt = std::fs::read_to_string(&prompt_path)?;
    let removed = prompt.replace("$orchestration", "Codexy orchestration");
    assert_ne!(removed, prompt, "fixture mutation must change the prompt");
    std::fs::write(
        &prompt_path,
        removed,
    )?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("interface.default_prompt must route through"));
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_top_level_prompt_metadata() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    std::fs::remove_file(plugin_root.join("agents/openai.yaml"))?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("agents/openai.yaml is required"));
    Ok(())
}

#[test]
fn validator_cli_rejects_skill_frontmatter_identity_mismatch() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        skill.replacen("name: engineering", "name: wrong-identity", 1),
    )?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("frontmatter name must match skill directory"));
    Ok(())
}

#[test]
fn validator_cli_accepts_valid_skill_frontmatter() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        skill.replacen(
            "description: Codexy plugin GitHub issue, branch, worktree, push, pull request, verification, repository-settings, branch-protection, review-thread resolution, and squash-merge workflow. MUST use before Git, issue, PR, label, review, protection, merge, or post-merge sync work in this repository.",
            "description: >\n  Structured YAML frontmatter can use\n  folded scalar values.",
            1,
        ),
    )?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_skill_frontmatter() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(&skill_path, skill.trim_start_matches("---\n"))?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("frontmatter must open with ---"));
    Ok(())
}

#[test]
fn validator_cli_rejects_malformed_skill_frontmatter() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        skill.replacen("name: engineering", "name: 'engineering", 1),
    )?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("frontmatter must be valid YAML"));
    Ok(())
}

#[test]
fn validator_cli_rejects_skill_frontmatter_without_closing_delimiter() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(&skill_path, skill.replacen("\n---\n", "\n...\n", 1))?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("frontmatter must close with ---"));
    Ok(())
}

#[test]
fn validator_cli_rejects_empty_skill_frontmatter_fields() -> TestResult {
    for (field, replacement, expected) in [
        ("name: engineering", "name: ", "frontmatter.name must be a non-empty string"),
        ("description:", "description: #", "frontmatter.description must be a non-empty string"),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_fixture(&plugin_root)?;
        let skill_path = plugin_root.join("skills/engineering/SKILL.md");
        let skill = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, skill.replacen(field, replacement, 1))?;
        let output = validator(&plugin_root, "--check-roles")?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains(expected));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_skill_document() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    std::fs::remove_file(plugin_root.join("skills/engineering/SKILL.md"))?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("skill bundle is missing SKILL.md"));
    Ok(())
}

#[test]
fn validator_cli_rejects_tab_indented_prompt_yaml() -> TestResult {
    assert_prompt_indent_rejected("  display_name:", "\tdisplay_name:")
}

#[test]
fn validator_cli_rejects_mixed_space_tab_prompt_yaml() -> TestResult {
    assert_prompt_indent_rejected("  display_name:", " \tdisplay_name:")
}

fn assert_prompt_indent_rejected(needle: &str, replacement: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let prompt = std::fs::read_to_string(&prompt_path)?;
    std::fs::write(&prompt_path, prompt.replace(needle, replacement))?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("must not contain tab indentation"));
    Ok(())
}

fn copy_fixture(plugin_root: &std::path::Path) -> std::io::Result<()> {
    copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        plugin_root,
    )
}

fn validator(
    plugin_root: &std::path::Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            mode,
        ])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
