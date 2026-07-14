use super::{TestResult, fixture, regular_lines, regular_lines_from, stderr, validate, write};

const SKILL_PATH: &str = "plugins/codexy/skills/example/SKILL.md";

#[test]
fn touched_loc_collects_every_same_line_skill_reference() -> TestResult {
    let repo = fixture(SKILL_PATH, regular_lines(252))?;
    write(
        repo.path(),
        SKILL_PATH,
        "# Example\n\n- [Overview](references/overview.md) [Workflow](references/workflow.md)\n",
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
fn touched_loc_ignores_unsafe_target_while_collecting_valid_same_line_reference() -> TestResult {
    let repo = fixture(SKILL_PATH, regular_lines(252))?;
    write(
        repo.path(),
        SKILL_PATH,
        "# Example\n\n- [Outside](../outside.md) [Workflow](references/workflow.md)\n",
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
