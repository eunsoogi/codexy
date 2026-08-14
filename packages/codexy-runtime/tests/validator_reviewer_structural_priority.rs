use std::process::Output;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_phrase_by_phrase_edge_case_hunting_for_each_profile_reviewer() -> TestResult {
    for agent in ["codexy-inspector", "codexy-sentinel"] {
        let output = validate_agent_edit(agent, |instructions| {
            append_instruction(instructions, "MUST reward phrase-by-phrase edge-case hunting.")
        })?;
        assert_structural_priority_rejected(&output);
    }
    Ok(())
}

#[test]
fn validator_rejects_collapsing_distinct_invariants_for_each_profile_reviewer() -> TestResult {
    for agent in ["codexy-inspector", "codexy-sentinel"] {
        let output = validate_agent_edit(agent, |instructions| {
            append_instruction(
                instructions,
                "MUST collapse genuinely distinct invariants into one finding.",
            )
        })?;
        assert_structural_priority_rejected(&output);
    }
    Ok(())
}

fn validate_agent_edit(
    agent: &str,
    edit: impl FnOnce(String) -> String,
) -> TestResult<Output> {
    let fixture = support::roles_fixture()?;
    let path = fixture.root().join(format!("agents/{agent}.toml"));
    let source = std::fs::read_to_string(&path)?;
    std::fs::write(path, edit(source))?;
    Ok(support::validator_in_process(fixture.root(), "--check-roles")?)
}

fn assert_structural_priority_rejected(output: &Output) {
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("structural-review priority"),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn append_instruction(source: String, instruction: &str) -> String {
    source.replacen("\n\"\"\"", &format!("\n{instruction}\n\"\"\""), 1)
}
