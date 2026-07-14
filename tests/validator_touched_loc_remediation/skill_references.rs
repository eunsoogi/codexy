use super::{TestResult, fixture, regular_lines, regular_lines_from, stderr, validate, write};

const SKILL_PATH: &str = "plugins/codexy/skills/example/SKILL.md";

#[test]
fn touched_loc_allows_skill_facade_extraction_into_one_linked_reference() -> TestResult {
    let repo = fixture(SKILL_PATH, regular_lines(252))?;
    write(
        repo.path(),
        SKILL_PATH,
        &format!(
            "# Example\n\n- [Workflow](references/workflow.md)\n\n{}",
            regular_lines_from(250, 2)
        ),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/references/workflow.md",
        &regular_lines(250),
    )?;

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_allows_skill_facade_extraction_into_linked_references() -> TestResult {
    let repo = fixture(SKILL_PATH, regular_lines(252))?;
    write(
        repo.path(),
        SKILL_PATH,
        "# Example\n\n- [Overview](references/overview.md)\n- [Workflow](references/workflow.md)\n",
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/references/overview.md",
        &regular_lines_from(0, 126),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/references/workflow.md",
        &regular_lines_from(126, 126),
    )?;

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_ignores_arbitrary_skill_sibling_files() -> TestResult {
    let repo = fixture(SKILL_PATH, regular_lines(252))?;
    write(
        repo.path(),
        SKILL_PATH,
        "# Example\n\n- [Overview](overview.md)\n- [Workflow](workflow.md)\n",
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/overview.md",
        &regular_lines_from(0, 126),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/workflow.md",
        &regular_lines_from(126, 126),
    )?;

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_ignores_unlinked_canonical_skill_reference() -> TestResult {
    let repo = fixture(SKILL_PATH, regular_lines(252))?;
    write(
        repo.path(),
        SKILL_PATH,
        &format!(
            "# Example\n\n- [Workflow](references/workflow.md)\n\n{}",
            regular_lines_from(250, 2)
        ),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/references/workflow.md",
        "# Workflow\n",
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/references/unlinked.md",
        &regular_lines(250),
    )?;

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}
