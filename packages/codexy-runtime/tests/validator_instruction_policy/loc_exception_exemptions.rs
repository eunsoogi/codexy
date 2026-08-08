use std::path::Path;

use super::{TestResult, stderr};
use crate::support::{instruction_policy_fixture, validator_instruction_policy_file};

const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];

#[test]
fn validator_cli_rejects_generic_passive_exempted_allowance() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let fixture = instruction_policy_fixture(Path::new(skill))?;
        let skill_path = fixture.path();
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            format!("{text}\n- Governed files are exempted from the 250 LOC limit.\n"),
        )?;

        let output = validator_instruction_policy_file(skill_path)?;
        assert!(!output.status.success(), "{skill:?} unexpectedly passed");
        assert!(stderr(&output).contains("LOC exception policy"));
    }
    Ok(())
}

#[test]
fn validator_cli_allows_negated_passive_loc_exception_wording() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let fixture = instruction_policy_fixture(Path::new(skill))?;
        let skill_path = fixture.path();
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            format!(
                "{text}\n- LOC exceptions MUST NOT be allowed or authorized.\n- LOC exceptions are not acceptable.\n- LOC exceptions are not exempt from the 250 LOC limit.\n- LOC exceptions MUST NOT be exempted from the 250 LOC limit.\n- Governed files MUST NOT be exempted from the 250 LOC limit.\n"
            ),
        )?;

        let output = validator_instruction_policy_file(skill_path)?;
        assert!(output.status.success(), "{skill:?}: {}", stderr(&output));
    }
    Ok(())
}
