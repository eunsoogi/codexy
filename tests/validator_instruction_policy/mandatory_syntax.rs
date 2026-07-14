use super::{TestResult, copy_plugin_fixture, stderr, validator};

#[test]
fn validator_cli_rejects_same_line_prohibition_with_must_not_elsewhere() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\n- Do not edit files; this line mentions MUST NOT as policy text.\n");
    std::fs::write(&skill_path, skill)?;
    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    Ok(())
}

#[test]
fn validator_cli_rejects_lowercase_must_and_must_not() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/proof-driven-completion/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\n- must run `git diff --check`.\n- must not edit files.\n");
    std::fs::write(&skill_path, skill)?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("mandatory instructions must use MUST"));
    assert!(stderr.contains("prohibitions must use MUST NOT"));
    Ok(())
}
