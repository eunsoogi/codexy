use std::{path::Path, process::Command};

use codexy_runtime::validation;

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
    let fixture = support::instruction_policy_fixture(Path::new(REFERENCE))?;
    let reference_path = fixture.path();
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
        assert_policy_file(
            &reference_path,
            false,
            &format!("connector policy regression escaped at case {index}"),
        )?;
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
        let fixture = support::instruction_policy_fixture(Path::new(REFERENCE))?;
        let reference_path = fixture.path();
        let reference = std::fs::read_to_string(&reference_path)?;
        std::fs::write(&reference_path, format!("{reference}\n{variant}\n"))?;
        assert_policy_file(
            &reference_path,
            false,
            &format!("connector request variant escaped: {variant:?}"),
        )?;
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
        let fixture = support::instruction_policy_fixture(Path::new(REFERENCE))?;
        let reference_path = fixture.path();
        let reference = std::fs::read_to_string(&reference_path)?;
        std::fs::write(&reference_path, format!("{reference}\n{control}\n"))?;
        assert_policy_file(
            &reference_path,
            true,
            &format!("valid prohibition was rejected: {control:?}"),
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_active_variants_on_every_governed_policy_surface() -> TestResult {
    for relative in [
        "AGENTS.md",
        "plugins/codexy/skills/git-workflow/SKILL.md",
        "plugins/codexy/skills/git-workflow/references/codex-connector-review.md",
    ] {
        for directive in [
            "MUST request connector review on every push.",
            "MUST enable automatic Codex connector review.",
            "The parent/orchestrator MUST configure automatic Codex connector review.",
        ] {
            let (_temp, plugin_root) = repo_fixture()?;
            let path = plugin_root
                .parent()
                .and_then(Path::parent)
                .ok_or("repository fixture root")?
                .join(relative);
            let original = std::fs::read_to_string(&path)?;
            std::fs::write(&path, format!("{original}\n{directive}\n"))?;
            assert_policy_rejected(
                &path,
                &format!("active directive escaped on {relative}: {directive:?}"),
            )?;
        }
    }
    Ok(())
}

#[test]
fn validator_ignores_excluded_policy_contexts_and_rejects_excluded_catalogs() -> TestResult {
    for excluded in [
        "> MUST request connector review on every push.",
        "```text\nMUST enable automatic Codex connector review.\n```",
        "### Historical example\nMUST request connector review on every push.",
        "### Inactive policy\nMUST configure automatic Codex connector review.",
        "Example: \"MUST request connector review on every push.\"",
    ] {
        let (_temp, plugin_root) = repo_fixture()?;
        let path = plugin_root.join(REFERENCE);
        let original = std::fs::read_to_string(&path)?;
        std::fs::write(&path, format!("{original}\n{excluded}\n"))?;
        assert_policy_allowed(
            &path,
            &format!("excluded context was treated as active: {excluded:?}"),
        )?;
    }

    for prefix in ["> ", "### Historical example\n"] {
        let (_temp, plugin_root) = repo_fixture()?;
        let path = plugin_root.join(REFERENCE);
        let original = std::fs::read_to_string(&path)?;
        let catalog = original
            .lines()
            .filter(|line| {
                line.chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_digit())
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(
            &path,
            format!(
                "# Manual Codex Connector Review\n\n## Required Procedure\n\n{prefix}{catalog}\n"
            ),
        )?;
        assert_policy_rejected(
            &path,
            &format!("excluded catalog satisfied active obligations: {prefix:?}"),
        )?;
    }
    Ok(())
}

fn plugin_fixture() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    Ok(support::copy_plugin_fixture_with_mutable_files(&[
        Path::new(REFERENCE),
    ])?)
}

fn repo_fixture() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    let (temp, source_plugin_root) = support::copy_plugin_fixture_with_mutable_files(&[
        Path::new("skills/git-workflow/SKILL.md"),
        Path::new(REFERENCE),
    ])?;
    let repo_root = temp.path().join("repo");
    let plugin_root = repo_root.join("plugins/codexy");
    std::fs::create_dir_all(repo_root.join("plugins"))?;
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md"),
        repo_root.join("AGENTS.md"),
    )?;
    std::fs::rename(source_plugin_root, &plugin_root)?;
    support::materialize_admission_runtime_suite(&plugin_root)?;
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

fn assert_policy_allowed(path: &Path, context: &str) -> TestResult {
    let diagnostics = validation::instruction_policy_diagnostics(path)?;
    assert!(diagnostics.is_empty(), "{context}: {diagnostics:#?}");
    Ok(())
}

fn assert_policy_file(path: &Path, expected: bool, context: &str) -> TestResult {
    let output = support::validator_instruction_policy_file(path)?;
    assert!(
        output.status.success() == expected,
        "{context}: expected {expected}, got {}",
        support::stderr(&output)
    );
    Ok(())
}

fn assert_policy_rejected(path: &Path, context: &str) -> TestResult {
    let diagnostics = validation::instruction_policy_diagnostics(path)?;
    assert!(!diagnostics.is_empty(), "{context}");
    Ok(())
}
