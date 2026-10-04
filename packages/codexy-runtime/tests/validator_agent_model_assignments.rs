use std::collections::BTreeSet;
use std::path::Path;

use crate::support;

#[path = "validator_agent_model_assignments/privacy.rs"]
mod privacy;

use support::{
    TestResult, agent_fixture, catalog_fixture, validate_agent_replacement,
    validate_catalog_replacement,
};

#[derive(Debug)]
struct ExpectedAgent {
    name: &'static str,
    filename: &'static str,
    model: &'static str,
    effort: &'static str,
}

const EXPECTED_AGENTS: &[ExpectedAgent] = &[ // Independent of packaged declarations.
    ExpectedAgent {
        name: "codexy-architect",
        filename: "codexy-architect.toml",
        model: "gpt-6-astra",
        effort: "high",
    },
    ExpectedAgent {
        name: "codexy-auditor",
        filename: "codexy-auditor.toml",
        model: "gpt-6.1-sol",
        effort: "medium",
    },
    ExpectedAgent {
        name: "codexy-cartographer",
        filename: "codexy-cartographer.toml",
        model: "gpt-6-luna",
        effort: "low",
    },
    ExpectedAgent {
        name: "codexy-inspector",
        filename: "codexy-inspector.toml",
        model: "gpt-6.1-sol",
        effort: "medium",
    },
    ExpectedAgent {
        name: "codexy-sentinel",
        filename: "codexy-sentinel.toml",
        model: "gpt-6-astra",
        effort: "xhigh",
    },
    ExpectedAgent {
        name: "codexy-shipwright",
        filename: "codexy-shipwright.toml",
        model: "gpt-6.1-sol",
        effort: "high",
    },
    ExpectedAgent {
        name: "codexy-warden",
        filename: "codexy-warden.toml",
        model: "gpt-6-astra",
        effort: "xhigh",
    },
    ExpectedAgent {
        name: "codexy-watcher",
        filename: "codexy-watcher.toml",
        model: "gpt-6-luna",
        effort: "max",
    },
];

#[test]
fn packaged_agents_match_the_independent_role_contract() -> TestResult {
    let agents_root = codexy_runtime::paths::repository_root().join("plugins/codexy/agents");
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
fn github_weaver_preserves_its_sol_model_and_reasoning_contract() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let agent = parse_agent(&root.join("plugins/codexy-github/agents/codexy-weaver.toml"))?;
    assert_eq!(agent["model"].as_str(), Some("gpt-6.1-sol"));
    assert_eq!(agent["model_reasoning_effort"].as_str(), Some("medium"));
    Ok(())
}

#[test]
fn validator_rejects_previous_sol_for_upgraded_roles() -> TestResult {
    let fixture = agent_fixture(EXPECTED_AGENTS.iter().map(|expected| expected.filename))?;
    for expected in EXPECTED_AGENTS
        .iter()
        .filter(|agent| agent.model == "gpt-6.1-sol")
    {
        assert_rejected(
            validate_agent_replacement(
                &fixture,
                expected.filename,
                "model",
                expected.model,
                "gpt-6-sol",
            )?,
            &format!("{} model must be {}", expected.name, expected.model),
        );
    }
    Ok(())
}

#[test]
fn standard_review_routes_to_the_upgraded_inspector() -> TestResult {
    use serde_json::json;
    let triggers = [
        "destructive",
        "security",
        "permission",
        "secret",
        "release",
        "high_consequence_external_state",
        "high_risk_guardrail",
        "merge_sensitive",
        "durable_delegation",
        "multi_lane_ownership",
        "explicit_audit_evidence",
    ];
    let request = json!({
        "schema": "codexy.review-profile-request.v1",
        "classification": {
            "schema": "codexy.workflow-profile-classification.v2",
            "work_class": "middle", "low_risk_eligible": false,
            "strict_triggers": triggers.map(|kind| json!({"kind": kind, "applies": false})),
        },
    });
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let route = codexy_runtime::validation::resolve_review_profile(&root, &request.to_string())?;
    assert_eq!(route["profile"], "standard");
    assert_eq!(
        route["reviewer"],
        json!({
            "name": "codexy-inspector", "model": "gpt-6.1-sol", "reasoning_effort": "medium",
        })
    );
    Ok(())
}

#[test]
fn validator_in_process_rejects_every_role_model_regression() -> TestResult {
    let fixture = agent_fixture(EXPECTED_AGENTS.iter().map(|expected| expected.filename))?;
    for expected in EXPECTED_AGENTS {
        assert_rejected(
            validate_agent_replacement(
                &fixture,
                expected.filename,
                "model",
                expected.model,
                "gpt-5.5",
            )?,
            &format!("{} model must be {}", expected.name, expected.model),
        );
    }
    Ok(())
}

#[test]
fn validator_in_process_rejects_every_role_effort_regression() -> TestResult {
    let fixture = agent_fixture(EXPECTED_AGENTS.iter().map(|expected| expected.filename))?;
    for expected in EXPECTED_AGENTS {
        assert_rejected(
            validate_agent_replacement(
                &fixture,
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
fn validator_in_process_reports_missing_catalog_contract_entry() -> TestResult {
    let fixture = catalog_fixture()?;
    assert_rejected(
        validate_catalog_replacement(&fixture, "  \"codexy-architect.toml\",\n", "")?,
        "missing: codexy-architect.toml; unexpected: none",
    );
    Ok(())
}

#[test]
fn validator_in_process_reports_unexpected_catalog_contract_entry() -> TestResult {
    let fixture = catalog_fixture()?;
    assert_rejected(
        validate_catalog_replacement(
            &fixture,
            "  \"codexy-watcher.toml\",\n]",
            "  \"codexy-watcher.toml\",\n  \"codexy-unknown.toml\",\n]",
        )?,
        "missing: none; unexpected: codexy-unknown.toml",
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
