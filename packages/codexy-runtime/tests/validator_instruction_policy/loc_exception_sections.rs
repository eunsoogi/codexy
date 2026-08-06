use std::path::Path;

use super::{TestResult, stderr};
use crate::support::{instruction_policy_fixture, validator_instruction_policy_file};

const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/plugin-marketplace-prep/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];

#[test]
fn validator_cli_rejects_allowances_later_in_loc_exception_sections() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let fixture = instruction_policy_fixture(Path::new(skill))?;
        for section in [
            "## LOC exceptions\n\n- Review requirements apply.\n- Allowed after approval.",
            "## LOC exceptions\n\n- Review requirements apply.\n\n### Approval workflow\n\n- Allowed after approval.",
        ] {
            fixture.reset()?;
            let skill_path = fixture.path();
            let text = std::fs::read_to_string(&skill_path)?;
            std::fs::write(&skill_path, format!("{text}\n{section}\n"))?;

            let output = validator_instruction_policy_file(skill_path)?;
            assert!(
                !output.status.success(),
                "{skill:?}: {section:?} unexpectedly passed"
            );
            assert!(stderr(&output).contains("LOC exception policy"));
        }
    }
    Ok(())
}

#[test]
fn validator_cli_resets_loc_exception_context_at_section_boundaries() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let fixture = instruction_policy_fixture(Path::new(skill))?;
        for boundary in ["## Review workflow", "# Review workflow"] {
            fixture.reset()?;
            let skill_path = fixture.path();
            let text = std::fs::read_to_string(&skill_path)?;
            std::fs::write(
                &skill_path,
                format!(
                    "{text}\n## LOC exceptions\n\n- LOC exceptions MUST NOT be allowed.\n\n{boundary}\n\n- Allowed after approval.\n"
                ),
            )?;

            let output = validator_instruction_policy_file(skill_path)?;
            assert!(output.status.success(), "{skill:?}: {}", stderr(&output));
        }
    }
    Ok(())
}
