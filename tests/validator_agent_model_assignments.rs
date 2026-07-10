use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

mod support;

use support::{
    TestResult, assert_privacy_diagnostic, public_contract_import_check,
    validate_agent_replacement, validate_catalog_replacement,
};

#[derive(Debug)]
struct ExpectedAgent {
    name: &'static str,
    filename: &'static str,
    model: &'static str,
    effort: &'static str,
}

const EXPECTED_AGENTS: &[ExpectedAgent] = &[
    ExpectedAgent {
        name: "codexy-architect",
        filename: "codexy-architect.toml",
        model: "gpt-5.6-sol",
        effort: "high",
    },
    ExpectedAgent {
        name: "codexy-auditor",
        filename: "codexy-auditor.toml",
        model: "gpt-5.6-terra",
        effort: "medium",
    },
    ExpectedAgent {
        name: "codexy-cartographer",
        filename: "codexy-cartographer.toml",
        model: "gpt-5.6-luna",
        effort: "low",
    },
    ExpectedAgent {
        name: "codexy-forge",
        filename: "codexy-forge.toml",
        model: "gpt-5.6-terra",
        effort: "medium",
    },
    ExpectedAgent {
        name: "codexy-pathfinder",
        filename: "codexy-pathfinder.toml",
        model: "gpt-5.6-sol",
        effort: "xhigh",
    },
    ExpectedAgent {
        name: "codexy-scribe",
        filename: "codexy-scribe.toml",
        model: "gpt-5.6-luna",
        effort: "low",
    },
    ExpectedAgent {
        name: "codexy-sculptor",
        filename: "codexy-sculptor.toml",
        model: "gpt-5.6-terra",
        effort: "high",
    },
    ExpectedAgent {
        name: "codexy-sentinel",
        filename: "codexy-sentinel.toml",
        model: "gpt-5.6-sol",
        effort: "xhigh",
    },
    ExpectedAgent {
        name: "codexy-shipwright",
        filename: "codexy-shipwright.toml",
        model: "gpt-5.6-terra",
        effort: "high",
    },
    ExpectedAgent {
        name: "codexy-tracer",
        filename: "codexy-tracer.toml",
        model: "gpt-5.6-sol",
        effort: "high",
    },
    ExpectedAgent {
        name: "codexy-warden",
        filename: "codexy-warden.toml",
        model: "gpt-5.6-sol",
        effort: "xhigh",
    },
    ExpectedAgent {
        name: "codexy-weaver",
        filename: "codexy-weaver.toml",
        model: "gpt-5.6-terra",
        effort: "medium",
    },
];

#[test]
fn packaged_agents_match_the_independent_role_contract() -> TestResult {
    let agents_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/agents");
    let actual_files = std::fs::read_dir(&agents_root)?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("codexy-") && name.ends_with(".toml"))
        .collect::<BTreeSet<_>>();
    let expected_files = EXPECTED_AGENTS
        .iter()
        .map(|agent| agent.filename.to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual_files, expected_files,
        "specialist file set must be exact"
    );
    for expected in EXPECTED_AGENTS {
        let agent = parse_agent(&agents_root.join(expected.filename))?;
        assert_eq!(
            agent.get("name").and_then(toml::Value::as_str),
            Some(expected.name)
        );
        assert_eq!(
            agent.get("model").and_then(toml::Value::as_str),
            Some(expected.model)
        );
        assert_eq!(
            agent
                .get("model_reasoning_effort")
                .and_then(toml::Value::as_str),
            Some(expected.effort)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_every_role_model_regression() -> TestResult {
    for expected in EXPECTED_AGENTS {
        assert_rejected(
            validate_agent_replacement(expected.filename, "model", expected.model, "gpt-5.5")?,
            &format!("{} model must be {}", expected.name, expected.model),
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_every_role_effort_regression() -> TestResult {
    for expected in EXPECTED_AGENTS {
        assert_rejected(
            validate_agent_replacement(
                expected.filename,
                "model_reasoning_effort",
                expected.effort,
                "ultra",
            )?,
            &format!(
                "{} model_reasoning_effort must be {}",
                expected.name, expected.effort
            ),
        );
    }
    Ok(())
}

#[test]
fn validator_cli_reports_missing_catalog_contract_entry() -> TestResult {
    assert_rejected(
        validate_catalog_replacement("  \"codexy-architect.toml\",\n", "")?,
        "missing: codexy-architect.toml; unexpected: none",
    );
    Ok(())
}

#[test]
fn validator_cli_reports_unexpected_catalog_contract_entry() -> TestResult {
    assert_rejected(
        validate_catalog_replacement(
            "  \"codexy-warden.toml\",\n]",
            "  \"codexy-warden.toml\",\n  \"codexy-unknown.toml\",\n]",
        )?,
        "missing: none; unexpected: codexy-unknown.toml",
    );
    Ok(())
}

#[test]
fn sentinel_uses_sol_with_xhigh_reasoning_and_not_ultra() -> TestResult {
    let sentinel = parse_agent(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/agents/codexy-sentinel.toml"),
    )?;
    assert_eq!(
        sentinel.get("model").and_then(toml::Value::as_str),
        Some("gpt-5.6-sol")
    );
    assert_eq!(
        sentinel
            .get("model_reasoning_effort")
            .and_then(toml::Value::as_str),
        Some("xhigh")
    );
    assert_ne!(
        sentinel
            .get("model_reasoning_effort")
            .and_then(toml::Value::as_str),
        Some("ultra")
    );
    Ok(())
}

#[test]
fn specialist_model_contract_is_not_a_public_api() -> TestResult {
    let output = public_contract_import_check()?;
    assert_privacy_diagnostic(&output)?;
    Ok(())
}

#[test]
fn privacy_contract_import_rejects_unrelated_cargo_failures() -> TestResult {
    let temp = tempfile::tempdir()?;
    let output = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(temp.path())
        .output()?;

    assert!(
        assert_privacy_diagnostic(&output).is_err(),
        "an unrelated cargo failure must not prove the specialist contract private"
    );
    Ok(())
}

fn parse_agent(path: &Path) -> TestResult<toml::Value> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}

fn assert_rejected(output: std::process::Output, expected: &str) {
    assert!(!output.status.success(), "validator unexpectedly succeeded");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
