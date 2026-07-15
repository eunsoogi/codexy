use super::{TestResult, copy_plugin_fixture, stderr, validator};

const SKILL: &str = "skills/git-workflow/SKILL.md";

#[test]
fn validator_allows_unrelated_permissions_after_a_non_heading_loc_prohibition() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join(SKILL);
    let text = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        format!("{text}\n- LOC exceptions MUST NOT be used.\n- Reviewers MAY approve labels.\n"),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_not_every_unconditional_loc_contract() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join(SKILL);
    let text = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        text.replace(
            "governed file MUST stay at or below 250 LOC",
            "Not every governed file MUST stay at or below 250 LOC",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains("missing unconditional governed 250 LOC clause"));
    Ok(())
}
