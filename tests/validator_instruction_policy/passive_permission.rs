use super::{TestResult, copy_plugin_fixture, copy_repo_fixture, stderr, validator};

const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/plugin-marketplace-prep/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];

#[test]
fn validator_rejects_authorized_loc_overage() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(skill);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            format!("{text}\n- A governed file is authorized to exceed 250 LOC.\n"),
        )?;

        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "{skill:?} unexpectedly passed");
        assert!(stderr(&output).contains("LOC exception policy"));
    }
    Ok(())
}

#[test]
fn validator_allows_negated_authorized_loc_overage() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(skill);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            format!("{text}\n- A governed file is not authorized to exceed 250 LOC.\n"),
        )?;

        let output = validator(&plugin_root, "--check")?;
        assert!(output.status.success(), "{skill:?}: {}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_rejects_authorized_loc_overage_in_governed_root_agents() -> TestResult {
    let (_temp, plugin_root, agents_path) = copy_repo_fixture()?;
    let agents = std::fs::read_to_string(&agents_path)?;
    std::fs::write(
        agents_path,
        format!("{agents}\n- A governed file is authorized to exceed 250 LOC.\n"),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(
        !output.status.success(),
        "root AGENTS.md unexpectedly passed"
    );
    assert!(stderr(&output).contains("LOC exception policy"));
    Ok(())
}

#[test]
fn validator_allows_negated_authorized_loc_overage_in_governed_root_agents() -> TestResult {
    let (_temp, plugin_root, agents_path) = copy_repo_fixture()?;
    let agents = std::fs::read_to_string(&agents_path)?;
    std::fs::write(
        agents_path,
        format!("{agents}\n- A governed file is not authorized to exceed 250 LOC.\n"),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(
        output.status.success(),
        "root AGENTS.md: {}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_rejects_non_adjacent_authorized_loc_overage() -> TestResult {
    for skill in GOVERNED_SKILLS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(skill);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(
            &skill_path,
            format!(
                "{text}\n- A governed file is authorized by maintainer approval to exceed 250 LOC.\n"
            ),
        )?;
        assert!(!validator(&plugin_root, "--check")?.status.success());
    }
    Ok(())
}

#[test]
fn validator_allows_safe_non_adjacent_authorization_observations() -> TestResult {
    for addition in [
        "A governed file is not authorized by maintainer approval to exceed 250 LOC.",
        "A governed file is authorized by maintainer approval to remain at or below 250 LOC.",
    ] {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(GOVERNED_SKILLS[0]);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, format!("{text}\n- {addition}\n"))?;
        assert!(
            validator(&plugin_root, "--check")?.status.success(),
            "{addition}"
        );
    }
    Ok(())
}

#[test]
fn validator_handles_waived_permissions_and_safe_observations() -> TestResult {
    for (addition, rejects) in [
        ("LOC exceptions are waived after approval.", true),
        ("LOC exceptions are not waived after approval.", false),
        ("LOC exceptions are approved after review.", true),
        ("LOC exceptions MUST NOT be approved after review.", false),
        ("Maintainers approve LOC exceptions after review.", true),
        (
            "Maintainers MUST NOT approve LOC exceptions after review.",
            false,
        ),
        ("The validator approved rejecting LOC exceptions.", false),
        (
            "Governed files are allowed to exceed 250 LOC with approval.",
            true,
        ),
        (
            "Governed files MUST NOT be allowed to exceed 250 LOC with approval.",
            false,
        ),
        (
            "The validator is authorized by maintainers to reject any governed file that exceeds 250 LOC.",
            false,
        ),
        (
            "A PR label waiver MAY be used when labels are disabled.",
            false,
        ),
        (
            "A PR label waiver MAY NOT be used when labels are disabled.",
            false,
        ),
        (
            "A waiver MAY exempt a governed file from the 250 LOC contract.",
            true,
        ),
        ("LOC exceptions require maintainer approval.", true),
        (
            "Maintainer approval MUST NOT authorize LOC exceptions.",
            false,
        ),
        (
            "The validator MUST require maintainer approval to reject LOC exceptions.",
            false,
        ),
        (
            "A governed file MAY go over 250 LOC with maintainer approval.",
            true,
        ),
        ("A governed file MAY be above 250 LOC.", true),
        ("A governed file MUST NOT go over 250 LOC.", false),
        ("A governed file MAY remain at or below 250 LOC.", false),
        (
            "The validator MAY reject governed files that go above 250 LOC.",
            false,
        ),
        ("LOC exceptions are granted after review.", true),
        ("LOC exceptions are not granted after review.", false),
        ("The validator granted rejecting LOC exceptions.", false),
    ] {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(GOVERNED_SKILLS[0]);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, format!("{text}\n- {addition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert_eq!(
            !output.status.success(),
            rejects,
            "{addition}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_handles_active_mandatory_permissions() -> TestResult {
    for (addition, rejects) in [
        (
            "Maintainers MUST authorize LOC exceptions after review.",
            true,
        ),
        (
            "Maintainers MUST NOT authorize LOC exceptions after review.",
            false,
        ),
        (
            "Maintainers MUST authorize rejecting LOC exceptions after review.",
            false,
        ),
        (
            "Maintainers MUST use LOC exceptions for approved overages.",
            true,
        ),
        (
            "Maintainers MUST NOT use LOC exceptions after review.",
            false,
        ),
        (
            "The validator MUST use LOC metrics to reject LOC exceptions.",
            false,
        ),
        (
            "The validator MUST use the 250 LOC below-limit check.",
            false,
        ),
        (
            "Maintainers MUST allow governed files to exceed 250 LOC with approval.",
            true,
        ),
        (
            "Maintainers MUST NOT allow governed files to exceed 250 LOC with approval.",
            false,
        ),
        (
            "Maintainers MUST allow governed files to remain at or below 250 LOC.",
            false,
        ),
    ] {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(GOVERNED_SKILLS[0]);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, format!("{text}\n- {addition}\n"))?;
        let output = validator(&plugin_root, "--check")?;
        assert_eq!(
            !output.status.success(),
            rejects,
            "{addition}: {}",
            stderr(&output)
        );
    }
    Ok(())
}
