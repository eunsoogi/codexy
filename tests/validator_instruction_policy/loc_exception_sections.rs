use super::{TestResult, copy_plugin_fixture, stderr, validator};

const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/plugin-marketplace-prep/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];

#[test]
fn validator_cli_rejects_allowances_later_in_loc_exception_sections() -> TestResult {
    for section in [
        "## LOC exceptions\n\n- Review requirements apply.\n- Allowed after approval.",
        "## LOC exceptions\n\n- Review requirements apply.\n\n### Approval workflow\n\n- Allowed after approval.",
    ] {
        for skill in GOVERNED_SKILLS {
            let (_temp, plugin_root) = copy_plugin_fixture()?;
            let skill_path = plugin_root.join(skill);
            let text = std::fs::read_to_string(&skill_path)?;
            std::fs::write(&skill_path, format!("{text}\n{section}\n"))?;

            let output = validator(&plugin_root, "--check")?;
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
    for boundary in ["## Review workflow", "# Review workflow"] {
        for skill in GOVERNED_SKILLS {
            let (_temp, plugin_root) = copy_plugin_fixture()?;
            let skill_path = plugin_root.join(skill);
            let text = std::fs::read_to_string(&skill_path)?;
            std::fs::write(
                &skill_path,
                format!(
                    "{text}\n## LOC exceptions\n\n- LOC exceptions MUST NOT be allowed.\n\n{boundary}\n\n- Allowed after approval.\n"
                ),
            )?;

            let output = validator(&plugin_root, "--check")?;
            assert!(output.status.success(), "{skill:?}: {}", stderr(&output));
        }
    }
    Ok(())
}
