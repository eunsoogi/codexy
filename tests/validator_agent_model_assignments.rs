use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};

use codexy_runtime::validation::agent_model_contract::SPECIALIST_MODEL_CONTRACTS;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_every_role_model_regression() -> TestResult {
    for contract in SPECIALIST_MODEL_CONTRACTS {
        let output = validate_agent_replacement(
            contract.name,
            &format!("model = {:?}", contract.model),
            "model = \"gpt-5.5\"",
        )?;
        assert!(
            !output.status.success(),
            "{} model regression passed",
            contract.name
        );
        assert!(
            stderr(&output).contains(&format!(
                "{} model must be {}",
                contract.name, contract.model
            )),
            "stderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_every_role_effort_regression() -> TestResult {
    for contract in SPECIALIST_MODEL_CONTRACTS {
        let output = validate_agent_replacement(
            contract.name,
            &format!("model_reasoning_effort = {:?}", contract.reasoning_effort),
            "model_reasoning_effort = \"ultra\"",
        )?;
        assert!(
            !output.status.success(),
            "{} effort regression passed",
            contract.name
        );
        assert!(
            stderr(&output).contains(&format!(
                "{} model_reasoning_effort must be {}",
                contract.name, contract.reasoning_effort
            )),
            "stderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_catalog_outside_exact_contract() -> TestResult {
    let output =
        validate_catalog_replacement("\"codexy-architect.toml\"", "\"codexy-unknown.toml\"")?;

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("agent_files must exactly match the specialist model contract")
    );
    Ok(())
}

#[test]
fn packaged_agents_use_role_appropriate_gpt_5_6_models() -> TestResult {
    let expected = SPECIALIST_MODEL_CONTRACTS
        .iter()
        .map(|contract| (contract.name, (contract.model, contract.reasoning_effort)))
        .collect::<BTreeMap<_, _>>();
    let agents_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/agents");
    let packaged = std::fs::read_dir(&agents_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("codexy-") && name.ends_with(".toml"))
        })
        .count();

    assert_eq!(
        packaged,
        expected.len(),
        "specialist mapping must be complete"
    );
    for (name, (expected_model, expected_effort)) in expected {
        let path = agents_root.join(format!("{name}.toml"));
        let agent = toml::from_str::<toml::Value>(&std::fs::read_to_string(&path)?)?;
        assert_eq!(
            agent.get("model").and_then(toml::Value::as_str),
            Some(expected_model),
            "{} must reject stale, non-5.6, or role-inappropriate model assignments",
            path.display()
        );
        assert_eq!(
            agent
                .get("model_reasoning_effort")
                .and_then(toml::Value::as_str),
            Some(expected_effort),
            "{} must use its role-appropriate reasoning effort",
            path.display()
        );
    }

    Ok(())
}

#[test]
fn sentinel_uses_sol_with_xhigh_reasoning_and_not_ultra() -> TestResult {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/agents/codexy-sentinel.toml");
    let agent = toml::from_str::<toml::Value>(&std::fs::read_to_string(path)?)?;

    assert_eq!(
        agent.get("model").and_then(toml::Value::as_str),
        Some("gpt-5.6-sol")
    );
    assert_eq!(
        agent
            .get("model_reasoning_effort")
            .and_then(toml::Value::as_str),
        Some("xhigh")
    );
    assert_ne!(
        agent
            .get("model_reasoning_effort")
            .and_then(toml::Value::as_str),
        Some("ultra")
    );

    Ok(())
}

fn validate_agent_replacement(name: &str, needle: &str, replacement: &str) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let path = plugin_root.join(format!("agents/{name}.toml"));
    let agent = std::fs::read_to_string(&path)?;
    std::fs::write(&path, agent.replacen(needle, replacement, 1))?;

    validator(&plugin_root)
}

fn validate_catalog_replacement(needle: &str, replacement: &str) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let path = plugin_root.join("agents/catalog.toml");
    let catalog = std::fs::read_to_string(&path)?;
    std::fs::write(&path, catalog.replacen(needle, replacement, 1))?;

    validator(&plugin_root)
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
