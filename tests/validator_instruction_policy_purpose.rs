use std::path::Path;
use std::process::Output;

use crate::support::{self, InstructionPolicyFixture};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_modal_purpose_clauses_with_prohibition_words() -> TestResult {
    let fixture = copy_plugin_fixture("skills/proof-driven-completion/SKILL.md")?;
    let skill_path = fixture.path();
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n- Evidence handoffs MUST include exact heads so future agents cannot confuse stale review output with current proof.\n",
    );
    skill.push_str("- When the check cannot run, MUST stop and report the blocker.\n");
    skill.push_str("- Review summaries MUST stop when the check cannot run.\n");
    skill.push_str("- Review summaries MUST name exact scope to avoid stale handoff claims.\n");
    std::fs::write(&skill_path, skill)?;

    let output = validator(skill_path, "--check")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_rejects_true_prohibitions_without_must_not() -> TestResult {
    let fixture = copy_plugin_fixture("skills/proof-driven-completion/SKILL.md")?;
    let skill_path = fixture.path();
    let skill = std::fs::read_to_string(&skill_path)?;
    for addition in [
        "- Evidence handoffs cannot omit exact heads.\n",
        "- Avoid stale handoff claims.\n",
        "- Review summaries MUST include exact scope, but cannot omit current proof.\n",
        "- Review summaries MUST include exact scope and avoid stale handoff claims.\n",
        "- Review summaries MUST include exact scope when available, but cannot omit current proof.\n",
        "- Review summaries MUST name exact scope to help reviewers and avoid stale handoff claims.\n",
        "- Review summaries MUST include exact scope when available. Cannot omit current proof.\n",
        "- Review summaries MUST include exact scope when available, cannot omit current proof.\n",
        "- Review summaries MUST include exact scope when available then cannot omit current proof.\n",
        "- Review summaries MUST report that stale output cannot prove current state, but avoid claiming stale output is current proof.\n",
    ] {
        std::fs::write(&skill_path, format!("{skill}\n{addition}"))?;
        let output = validator(skill_path, "--check")?;
        assert!(
            !output.status.success(),
            "addition {addition:?} unexpectedly passed"
        );
        assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    }
    Ok(())
}

#[test]
fn validator_allows_separate_must_action_after_prohibition() -> TestResult {
    let fixture = copy_plugin_fixture("skills/agents-md-authoring/SKILL.md")?;
    let skill_path = fixture.path();
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\n- MUST NOT leave temp servers running, and MUST add the cleanup receipt to the handoff.\n",
    );
    std::fs::write(&skill_path, skill)?;
    let output = validator(skill_path, "--check")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
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
