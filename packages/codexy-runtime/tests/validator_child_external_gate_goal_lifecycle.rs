use std::fs;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> TestResult<support::InstructionPolicyFixture> {
    Ok(support::instruction_policy_fixture(std::path::Path::new(
        "skills/orchestration/SKILL.md",
    ))?)
}

#[test]
fn validator_rejects_external_gate_completion_policy() -> TestResult {
    let fixture = fixture()?;
    let original = fs::read_to_string(fixture.path())?;
    fs::write(
        fixture.path(),
        format!(
            "{original}\nA child with only an external gate remaining MUST call update_goal(complete) before waiting.\n"
        ),
    )?;

    let output = support::validator_instruction_policy_file(fixture.path())?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("must not complete a goal for an external-gate wait"));
    Ok(())
}

#[test]
fn validator_rejects_ending_an_active_goal_without_a_transition() -> TestResult {
    let fixture = fixture()?;
    let original = fs::read_to_string(fixture.path())?;
    fs::write(
        fixture.path(),
        format!(
            "{original}\nA child external-gate wait MUST end its active goal and plan before waiting while goal transition=none.\n"
        ),
    )?;

    let output = support::validator_instruction_policy_file(fixture.path())?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("must retain an active goal during a nonterminal wait"));
    Ok(())
}
