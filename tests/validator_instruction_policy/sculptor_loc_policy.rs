use super::{TestResult, copy_plugin_fixture, stderr, validator};

const RATIONALE_MARKER: &str = "exceed the LOC target without a narrow rationale";
const RATIONALE_AUTHORIZATION: &str =
    "MUST stop and escalate when touched files exceed the LOC target without a narrow rationale.";
const SAFE_ESCALATION: &str = "MUST stop and escalate when touched files exceed the LOC target.";
const SAFE_PROHIBITION: &str =
    "MUST NOT allow touched files to exceed the LOC target even with a narrow rationale.";

#[test]
fn validator_cli_rejects_sculptor_rationale_based_overage_authorization() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let agent_path = plugin_root.join("agents/codexy-sculptor.toml");
    let agent = without_rationale_authorization(&std::fs::read_to_string(&agent_path)?);
    std::fs::write(
        &agent_path,
        inject_developer_instruction(&agent, RATIONALE_AUTHORIZATION),
    )?;

    let output = validator(&plugin_root, "--check-roles")?;
    assert!(
        !output.status.success(),
        "rationale-based Sculptor overage authorization unexpectedly passed"
    );
    let stderr = stderr(&output);
    assert!(stderr.contains("codexy-sculptor.toml"), "{stderr}");
    assert!(stderr.contains("must not allow LOC exceptions"), "{stderr}");
    Ok(())
}

#[test]
fn validator_cli_allows_sculptor_unconditional_overage_escalation() -> TestResult {
    for instruction in [SAFE_ESCALATION, SAFE_PROHIBITION] {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let agent_path = plugin_root.join("agents/codexy-sculptor.toml");
        let agent = without_rationale_authorization(&std::fs::read_to_string(&agent_path)?);
        std::fs::write(
            &agent_path,
            inject_developer_instruction(&agent, instruction),
        )?;

        let output = validator(&plugin_root, "--check-roles")?;
        assert!(output.status.success(), "{}", stderr(&output));
    }
    Ok(())
}

fn without_rationale_authorization(agent: &str) -> String {
    agent
        .lines()
        .filter(|line| !line.contains(RATIONALE_MARKER))
        .collect::<Vec<_>>()
        .join("\n")
}

fn inject_developer_instruction(agent: &str, instruction: &str) -> String {
    let closing = agent
        .rfind("\"\"\"")
        .expect("Sculptor developer instructions must remain multiline TOML");
    format!(
        "{}\n{instruction}\n{}",
        &agent[..closing],
        &agent[closing..]
    )
}
