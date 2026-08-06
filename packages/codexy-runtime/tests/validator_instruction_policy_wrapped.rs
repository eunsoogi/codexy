use std::path::Path;
use std::process::Output;

use crate::support::{self, InstructionPolicyFixture};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_wrapped_modal_continuation_prohibition() -> TestResult {
    let fixture = copy_plugin_fixture()?;
    let skill_path = fixture.path();
    let skill = std::fs::read_to_string(&skill_path)?;
    for addition in [
        "The agent MUST use codegraph output to\navoid direct edits.",
        "The agent MUST use codegraph output to\nDo not edit files.",
        "The agent MUST use codegraph output to\nidentify nearby files. Do not edit files.",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}\n"))?;

        let output = validator(skill_path, "--check")?;

        assert!(
            !output.status.success(),
            "instruction {addition:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_wrapped_modal_continuation_clause_imperatives() -> TestResult {
    let fixture = copy_plugin_fixture()?;
    let skill_path = fixture.path();
    let skill = std::fs::read_to_string(&skill_path)?;
    for addition in [
        "The agent MUST use codegraph output to\nidentify nearby files, run the validator.",
        "The agent MUST use codegraph output to\nidentify nearby files and run the validator.",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}\n"))?;

        let output = validator(skill_path, "--check")?;

        assert!(
            !output.status.success(),
            "instruction {addition:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_wrapped_modal_continuation_without_new_instruction() -> TestResult {
    let fixture = copy_plugin_fixture()?;
    let skill_path = fixture.path();
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        format!("{skill}\nThe agent MUST use codegraph output to\nidentify nearby files.\n"),
    )?;

    let output = validator(skill_path, "--check")?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn copy_plugin_fixture() -> TestResult<InstructionPolicyFixture> {
    Ok(support::instruction_policy_fixture(Path::new(
        "skills/proof-driven-completion/SKILL.md",
    ))?)
}

fn validator(path: &Path, mode: &str) -> TestResult<Output> {
    if mode == "--check" {
        return support::validator_instruction_policy_file(path);
    }
    Err(format!("unsupported focused validator mode {mode}").into())
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
