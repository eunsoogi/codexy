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
fn validator_cli_rejects_passive_loc_exception_allowances() -> TestResult {
    for allowance in [
        "LOC exceptions are allowed when approved.",
        "LOC exceptions are permitted after review.",
        "LOC exceptions are authorized by maintainers.",
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
fn validator_cli_allows_negated_passive_loc_exception_wording() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(skill);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            format!("{text}\n- LOC exceptions MUST NOT be allowed or authorized.\n"),
        )?;

        let output = validator(&plugin_root, "--check")?;
        assert!(output.status.success(), "{skill:?}: {}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_mixed_polarity_loc_exception_authorization() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(skill);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            format!(
                "{text}\n- LOC exceptions are not permitted but are authorized by maintainers.\n"
            ),
        )?;

        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "{skill:?} unexpectedly passed");
        assert!(stderr(&output).contains("LOC exception policy"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_loc_exception_carve_outs() -> TestResult {
    for carve_out in [
        "LOC exceptions MUST NOT be allowed except by maintainer approval.",
        "LOC exceptions MUST NOT be permitted except when security review approves them.",
        "LOC exceptions MUST NOT be authorized other than with maintainer approval.",
    ] {
        for skill in GOVERNED_SKILLS {
            let (_temp, plugin_root) = copy_plugin_fixture()?;
            let skill_path = plugin_root.join(skill);
            let text = std::fs::read_to_string(&skill_path)?;
            std::fs::write(&skill_path, format!("{text}\n- {carve_out}\n"))?;

            let output = validator(&plugin_root, "--check")?;
            assert!(
                !output.status.success(),
                "{skill:?}: {carve_out:?} unexpectedly passed"
            );
            assert!(stderr(&output).contains("LOC exception policy"));
        }
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_waiver_after_safe_prohibition_across_clause_boundaries() -> TestResult {
    for skill in GOVERNED_SKILLS {
        for clauses in [
            "MUST NOT collapse readable multiline code.\n- A governed file MAY exceed 250 LOC when a waiver is approved.",
            "MUST NOT collapse readable multiline code. A governed file MAY exceed 250 LOC when a waiver is approved.",
            "MUST NOT collapse readable multiline code; A governed file MAY exceed 250 LOC when a waiver is approved.",
            "MUST NOT collapse readable multiline code, but a governed file MAY exceed 250 LOC when a waiver is approved.",
        ] {
            let (_temp, plugin_root) = copy_plugin_fixture()?;
            let skill_path = plugin_root.join(skill);
            let text = std::fs::read_to_string(&skill_path)?;
            std::fs::write(
                &skill_path,
                format!(
                    "{text}\n- {clauses}\n- MUST NOT delete blank lines solely to meet the target.\n"
                ),
            )?;

            let output = validator(&plugin_root, "--check")?;
            assert!(
                !output.status.success(),
                "{skill:?}: {clauses:?} unexpectedly passed"
            );
            assert!(stderr(&output).contains("LOC exception policy"));
        }
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
fn validator_cli_rejects_negated_unconditional_loc_contract() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(skill);
        let text = std::fs::read_to_string(&skill_path)?;
        let text = text.replace(
            "MUST stay at or below 250 LOC",
            "MAY stay at or below 250 LOC",
        );
        std::fs::write(
            &skill_path,
            format!(
                "{text}\n- MUST NOT claim that every governed file MUST stay at or below 250 LOC.\n"
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
