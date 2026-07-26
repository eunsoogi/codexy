use std::{path::Path, process::Command};

#[path = "structured_contract.rs"]
mod structured_contract;

use crate::support;
use structured_contract::{Contract, Modality, Rule};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const REFERENCE: &str = "skills/git-workflow/references/codex-connector-review.md";

#[test]
fn repository_records_the_manual_connector_review_policy() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let agents = std::fs::read_to_string(root.join("AGENTS.md"))?;
    Contract::markdown(&agents)
        .assert_rule(Rule::new(
            "agents.connector-review.automatic-disabled",
            "Codex connector automatic review",
            Modality::Required,
            &["remain"],
            &["disabled"],
        ))
        .expect("root policy must retain disabled automatic connector review");
    Ok(())
}

#[test]
fn validator_accepts_the_required_manual_connector_review_contract() -> TestResult {
    let (_temp, plugin_root) = plugin_fixture()?;
    let output = validate(&plugin_root)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_missing_historical_or_fenced_connector_review_policy() -> TestResult {
    let (_temp, plugin_root) = plugin_fixture()?;
    let reference_path = plugin_root.join(REFERENCE);
    let reference = std::fs::read_to_string(&reference_path)?;

    for (index, policy) in [
        reference.replacen("[proof-ci-before-review] ", "", 1),
        reference.replacen(
            "[automatic-disabled] Codex connector automatic review MUST remain disabled.",
            "[automatic-disabled] Historical example: Codex connector automatic review MUST remain disabled.",
            1,
        ),
        reference.replacen(
            "[automatic-disabled] Codex connector automatic review MUST remain disabled.",
            "[automatic-disabled] ```text\nCodex connector automatic review MUST remain disabled.\n```",
            1,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        std::fs::write(&reference_path, policy)?;
        let output = validate(&plugin_root)?;
        assert!(
            !output.status.success(),
            "connector policy regression escaped at case {index}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::write(&reference_path, &reference)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_active_automatic_or_repeated_request_variants() -> TestResult {
    for variant in [
        "The parent/orchestrator MUST request connector review on every push.",
        "The parent/orchestrator MUST request connector review per-push.",
        "The parent/orchestrator MUST request an automatic connector review.",
        "The parent/orchestrator MUST request a duplicate connector review.",
        "The parent/orchestrator MUST request another connector review after repair.",
        "The parent/orchestrator MUST request a second connector review after repair.",
        "The parent/orchestrator MUST request piecemeal connector reviews.",
    ] {
        let (_temp, plugin_root) = plugin_fixture()?;
        let reference_path = plugin_root.join(REFERENCE);
        let reference = std::fs::read_to_string(&reference_path)?;
        std::fs::write(&reference_path, format!("{reference}\n{variant}\n"))?;
        let output = validate(&plugin_root)?;
        assert!(
            !output.status.success(),
            "connector request variant escaped: {variant:?}\\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_active_prohibitions_and_quoted_counterexamples() -> TestResult {
    for control in [
        "The parent/orchestrator MUST NOT request connector review on every push.",
        "The statement \"Automatic Codex connector review is enabled for every push.\" MUST NOT be permitted.",
        "A duplicate, second, or piecemeal connector review MUST NOT be requested.",
    ] {
        let (_temp, plugin_root) = plugin_fixture()?;
        let reference_path = plugin_root.join(REFERENCE);
        let reference = std::fs::read_to_string(&reference_path)?;
        std::fs::write(&reference_path, format!("{reference}\n{control}\n"))?;
        let output = validate(&plugin_root)?;
        assert!(
            output.status.success(),
            "valid prohibition was rejected: {control:?}\\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn plugin_fixture() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok((temp, plugin_root))
}

fn validate(plugin_root: &Path) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?)
}
