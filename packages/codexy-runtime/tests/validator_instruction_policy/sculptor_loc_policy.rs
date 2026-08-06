use std::path::{Path, PathBuf};

use super::{TestResult, stderr, validator};
use crate::support::{self, PluginFixture};

const SCULPTOR_AGENT: &str = "agents/codexy-sculptor.toml";

const RATIONALE_MARKER: &str = "exceed the LOC target without a narrow rationale";
const RATIONALE_AUTHORIZATION: &str =
    "MUST stop and escalate when touched files exceed the LOC target without a narrow rationale.";
const UNRELATED_NEGATION_AUTHORIZATION: &str = "MUST NOT collapse readable code, but touched files MAY exceed the LOC target without a narrow rationale.";
const UNRELATED_CONJUNCTION_NEGATION_AUTHORIZATION: &str = "MUST NOT collapse readable code, and touched files MAY exceed the LOC target without a narrow rationale.";
const SAFE_ESCALATION: &str = "MUST stop and escalate when touched files exceed the LOC target.";
const SAFE_PROHIBITION: &str =
    "MUST NOT allow touched files to exceed the LOC target even with a narrow rationale.";
const SAFE_POSTPOSED_PROHIBITION: &str =
    "Touched files exceeding the LOC target without a rationale MUST NOT be accepted.";
const DIRECT_OVERAGE_AUTHORIZATION: &str =
    "A governed file MAY exceed 250 LOC with maintainer approval.";
const PASSIVE_OVERAGE_AUTHORIZATION: &[&str] = &[
    "A governed file is allowed to exceed 250 LOC with maintainer approval.",
    "A governed file is permitted to exceed 250 LOC with maintainer approval.",
];
const SAFE_UNCONDITIONAL_PROHIBITION: &str = "A governed file MUST NOT exceed 250 LOC.";
const SAFE_NEGATED_PERMISSION: &str =
    "A governed file MAY NOT exceed 250 LOC with maintainer approval.";
const SAFE_NON_LOC_APPROVAL: &str = "A governed file MAY be reorganized with maintainer approval.";
const SAFE_NEGATED_PASSIVE_PERMISSION: &[&str] = &[
    "A governed file MUST NOT be allowed to exceed 250 LOC with maintainer approval.",
    "A governed file MUST NOT be permitted to exceed 250 LOC with maintainer approval.",
];
const SAFE_OVERAGE_OBSERVATIONS: &[&str] = &[
    "The validator MAY report when a governed file exceeds 250 LOC.",
    "The validator can detect whether a governed file exceeds 250 LOC.",
];

#[test]
fn validator_cli_rejects_sculptor_rationale_based_overage_authorization() -> TestResult {
    let fixture = sculptor_fixture()?;
    let plugin_root = fixture.root();
    for instruction in [
        RATIONALE_AUTHORIZATION,
        UNRELATED_NEGATION_AUTHORIZATION,
        UNRELATED_CONJUNCTION_NEGATION_AUTHORIZATION,
    ] {
        let (agent_path, agent) = reset_sculptor_agent(&fixture)?;
        std::fs::write(
            &agent_path,
            inject_developer_instruction(&agent, instruction),
        )?;

        let output = validator(plugin_root, "--check-roles")?;
        assert!(
            !output.status.success(),
            "rationale-based Sculptor overage authorization unexpectedly passed: {instruction}"
        );
        let stderr = stderr(&output);
        assert!(stderr.contains("codexy-sculptor.toml"), "{stderr}");
        assert!(stderr.contains("must not allow LOC exceptions"), "{stderr}");
    }
    Ok(())
}

#[test]
fn validator_cli_allows_sculptor_unconditional_overage_escalation() -> TestResult {
    let fixture = sculptor_fixture()?;
    let plugin_root = fixture.root();
    for instruction in [
        SAFE_ESCALATION,
        SAFE_PROHIBITION,
        SAFE_POSTPOSED_PROHIBITION,
        SAFE_UNCONDITIONAL_PROHIBITION,
        SAFE_NEGATED_PERMISSION,
        SAFE_NON_LOC_APPROVAL,
    ] {
        let (agent_path, agent) = reset_sculptor_agent(&fixture)?;
        std::fs::write(
            &agent_path,
            inject_developer_instruction(&agent, instruction),
        )?;

        let output = validator(plugin_root, "--check-roles")?;
        assert!(output.status.success(), "{}", stderr(&output));
    }
    for instruction in SAFE_NEGATED_PASSIVE_PERMISSION
        .iter()
        .chain(SAFE_OVERAGE_OBSERVATIONS)
    {
        let (agent_path, agent) = reset_sculptor_agent(&fixture)?;
        std::fs::write(
            &agent_path,
            inject_developer_instruction(&agent, instruction),
        )?;

        let output = validator(plugin_root, "--check-roles")?;
        assert!(
            output.status.success(),
            "safe instruction unexpectedly failed: {instruction}\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_direct_sculptor_loc_overage_permission() -> TestResult {
    let fixture = sculptor_fixture()?;
    let plugin_root = fixture.root();
    for instruction in
        std::iter::once(&DIRECT_OVERAGE_AUTHORIZATION).chain(PASSIVE_OVERAGE_AUTHORIZATION.iter())
    {
        let (agent_path, agent) = reset_sculptor_agent(&fixture)?;
        std::fs::write(
            &agent_path,
            inject_developer_instruction(&agent, instruction),
        )?;

        let output = validator(plugin_root, "--check-roles")?;
        assert!(
            !output.status.success(),
            "direct Sculptor LOC overage permission unexpectedly passed: {instruction}"
        );
        let stderr = stderr(&output);
        assert!(stderr.contains("codexy-sculptor.toml"), "{stderr}");
        assert!(stderr.contains("must not allow LOC exceptions"), "{stderr}");
    }
    Ok(())
}

fn sculptor_fixture() -> TestResult<PluginFixture> {
    Ok(support::plugin_fixture_with_mutable_files(&[Path::new(SCULPTOR_AGENT)])?)
}

fn reset_sculptor_agent(fixture: &PluginFixture) -> std::io::Result<(PathBuf, String)> {
    let relative = Path::new(SCULPTOR_AGENT);
    fixture.reset_file(relative)?;
    let path = fixture.root().join(relative);
    Ok((path.clone(), without_rationale_authorization(&std::fs::read_to_string(path)?)))
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
