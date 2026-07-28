use super::{TestResult, copy_repo_fixture, stderr, validator};

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
