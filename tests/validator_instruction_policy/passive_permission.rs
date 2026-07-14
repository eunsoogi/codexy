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
        (
            "The validator is authorized by maintainers to reject any governed file that exceeds 250 LOC.",
            false,
        ),
    ] {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join(GOVERNED_SKILLS[0]);
        let text = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, format!("{text}\n- {addition}\n"))?;
        assert_eq!(
            !validator(&plugin_root, "--check")?.status.success(),
            rejects,
            "{addition}"
        );
    }
    Ok(())
}
