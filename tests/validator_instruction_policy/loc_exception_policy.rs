use super::{TestResult, copy_plugin_fixture, stderr, validator};

const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/plugin-marketplace-prep/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];

#[test]
fn validator_cli_rejects_loc_exception_allowances_in_governed_skills() -> TestResult {
    for allowance in [
        "A tracked Codexy LOC exception MAY exempt a governed file from the 250 LOC contract.",
        "A governed file MAY exceed 250 LOC when a tracked waiver contains a narrow maintained rationale.",
        "## LOC exceptions\n\n- A tracked entry MAY exempt a governed file.",
    ] {
        for skill in GOVERNED_SKILLS {
            let (_temp, plugin_root) = copy_plugin_fixture()?;
            let skill_path = plugin_root.join(skill);
            let text = std::fs::read_to_string(&skill_path)?;
            std::fs::write(&skill_path, format!("{text}\n- {allowance}\n"))?;

            let output = validator(&plugin_root, "--check")?;
            assert!(
                !output.status.success(),
                "{skill:?}: {allowance:?} unexpectedly passed"
            );
            assert!(stderr(&output).contains("LOC exception policy"));
        }
    }
    Ok(())
}

#[test]
fn validator_cli_allows_negated_loc_exception_prohibition() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(skill);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            format!("{text}\n- MUST NOT allow LOC exceptions.\n"),
        )?;

        let output = validator(&plugin_root, "--check")?;
        assert!(output.status.success(), "{skill:?}: {}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_unconditional_loc_contract() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(skill);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            text.replace(
                "MUST stay at or below 250 LOC",
                "MAY stay at or below 250 LOC",
            ),
        )?;

        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "{skill:?} unexpectedly passed");
        assert!(stderr(&output).contains("missing unconditional governed 250 LOC clause"));
    }
    Ok(())
}

#[test]
fn current_refactoring_and_sentinel_surfaces_prohibit_exceptions() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let refactoring = std::fs::read_to_string(plugin_root.join("skills/refactoring/SKILL.md"))?;
    let sentinel = std::fs::read_to_string(plugin_root.join("agents/codexy-sentinel.toml"))?;
    assert!(!refactoring.contains("remaining large-file exceptions"));
    assert!(!refactoring.contains("Exceptions and rationale"));
    assert!(sentinel.contains("MUST block when any governed file exceeds 250 LOC"));
    assert!(!sentinel.contains("explicit narrow exception rationale"));
    Ok(())
}
