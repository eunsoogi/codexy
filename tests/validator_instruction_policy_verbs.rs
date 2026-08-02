use std::path::Path;
use std::process::Output;

use crate::support::{self, InstructionPolicyFixture};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_remaining_bare_imperative_verbs() -> TestResult {
    let fixture = copy_plugin_fixture("skills/proof-driven-completion/SKILL.md")?;
    let skill_path = fixture.path();
    let skill = std::fs::read_to_string(&skill_path)?;

    for addition in [
        "- Parse structured files before handoff.",
        "- Name the required Codexy skills.",
        "- Decide whether multi-agent helper work is useful.",
        "- Pull forward the wiki-level findings.",
        "- Open full records only when the user asks for detail.",
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
fn validator_cli_accepts_modal_wrapped_remaining_imperative_verbs() -> TestResult {
    let fixture = copy_plugin_fixture("skills/proof-driven-completion/SKILL.md")?;
    let skill_path = fixture.path();
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n- MUST parse structured files before handoff.\n\
         - MUST name the required Codexy skills.\n\
         - MUST decide whether multi-agent helper work is useful.\n\
         - MUST pull forward the wiki-level findings.\n\
         - MUST open full records only when the user asks for detail.\n",
    );
    std::fs::write(&skill_path, skill)?;

    let output = validator(skill_path, "--check")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_conditional_clause_bare_imperatives() -> TestResult {
    let fixture = copy_plugin_fixture("skills/proof-driven-completion/SKILL.md")?;
    let skill_path = fixture.path();
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        format!("{skill}\n- If verification fails, run the validator.\n"),
    )?;

    let output = validator(skill_path, "--check")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

#[test]
fn validator_cli_rejects_skill_description_bare_imperatives() -> TestResult {
    let fixture = copy_plugin_fixture("skills/task-classification/SKILL.md")?;
    let skill_path = fixture.path();
    let skill = std::fs::read_to_string(&skill_path)?;
    assert!(skill.contains("description: MUST use first"));
    std::fs::write(
        &skill_path,
        skill.replace("description: MUST use first", "description: Use first"),
    )?;

    let output = validator(skill_path, "--check")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

fn copy_plugin_fixture(relative: &str) -> TestResult<InstructionPolicyFixture> {
    Ok(support::instruction_policy_fixture(Path::new(relative))?)
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
