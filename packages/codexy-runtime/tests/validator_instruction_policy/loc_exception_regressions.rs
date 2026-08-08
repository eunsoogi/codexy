use std::path::Path;

use super::{TestResult, stderr};
use crate::support::{instruction_policy_fixture, validator_instruction_policy_file};

const SKILL: &str = "skills/git-workflow/SKILL.md";

#[test]
fn validator_allows_unrelated_permissions_after_a_non_heading_loc_prohibition() -> TestResult {
    let fixture = instruction_policy_fixture(Path::new(SKILL))?;
    let skill_path = fixture.path();
    let text = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        format!("{text}\n- LOC exceptions MUST NOT be used.\n- Reviewers MAY approve labels.\n"),
    )?;

    let output = validator_instruction_policy_file(skill_path)?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_not_every_unconditional_loc_contract() -> TestResult {
    let fixture = instruction_policy_fixture(Path::new(SKILL))?;
    let skill_path = fixture.path();
    let text = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        text.replace(
            "governed file MUST stay at or below 250 LOC",
            "Not every governed file MUST stay at or below 250 LOC",
        ),
    )?;

    let output = validator_instruction_policy_file(skill_path)?;
    assert!(!output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains("missing unconditional governed 250 LOC clause"));
    Ok(())
}
