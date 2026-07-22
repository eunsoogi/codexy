use std::{path::Path, process::Command};

use crate::support;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_reintroduced_connector_review_gate() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let skill_path = plugin_root.join("skills/git-workflow/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\n- MUST require Codex review before pull request readiness.\n");
    std::fs::write(&skill_path, skill)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Codex connector review policy is not allowed")
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_line_wrapped_connector_review_gate() -> TestResult {
    let (temp, plugin_root) = plugin_fixture()?;
    let skill_path = plugin_root.join("skills/git-workflow/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\n- MUST require Codex\n  review before pull request readiness.\n");
    std::fs::write(&skill_path, skill)?;

    let output = validate(&plugin_root)?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Codex connector review policy is not allowed")
    );
    drop(temp);
    Ok(())
}

#[test]
fn validator_cli_rejects_connector_specific_policy_forms() -> TestResult {
    for policy in [
        "MUST wait for Codex connector review before readiness.",
        "MUST capture Codex connector output before readiness.",
    ] {
        let (_temp, plugin_root) = plugin_fixture()?;
        let skill_path = plugin_root.join("skills/git-workflow/SKILL.md");
        let mut skill = std::fs::read_to_string(&skill_path)?;
        skill.push_str(&format!("\n- {policy}\n"));
        std::fs::write(&skill_path, skill)?;
        let output = validate(&plugin_root)?;
        assert!(!output.status.success(), "policy escaped guard: {policy}");
    }
    Ok(())
}

#[test]
fn validator_does_not_misclassify_packaged_codexy_reviewer_instruction() -> TestResult {
    let (_temp, plugin_root) = plugin_fixture()?;
    let skill_path = plugin_root.join("skills/git-workflow/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\n- MUST run the packaged Codexy reviewer agent.\n");
    std::fs::write(&skill_path, skill)?;
    let output = validate(&plugin_root)?;
    assert!(!output.status.success(), "the new MUST must remain uncovered");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("uncovered normative rules"));
    assert!(!stderr.contains("Codex connector review policy is not allowed"));
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
