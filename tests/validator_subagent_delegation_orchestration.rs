use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_recursive_permission_appended_to_canonical_child_clause() -> TestResult {
    for suffix in [
        "and those helpers MAY spawn another helper",
        "and delegate work to another helper",
        "and create another reviewer task",
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = fixture(&temp)?;
        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let mut skill = std::fs::read_to_string(&skill_path)?;
        skill.push_str(&format!(
            "\nA child implementation thread MAY spawn bounded first-level specialist helpers or Sentinel reviewers, {suffix}.\n",
        ));
        std::fs::write(&skill_path, skill)?;
        assert_recursion_rejected(validator(&plugin_root)?, suffix);
    }
    Ok(())
}

#[test]
fn validator_allows_orchestrator_child_thread_creation() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = fixture(&temp)?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\nThe root orchestrator MUST create a child thread.\n");
    skill.push_str("The root orchestrator MAY create child threads.\n");
    skill.push_str("The root orchestrator MUST start a child thread.\n");
    skill.push_str("The root orchestrator MUST fork a child thread.\n");
    skill.push_str("The root orchestrator MUST assign a child thread.\n");
    skill
        .push_str("The root orchestrator MUST notify a reviewer before creating a child thread.\n");
    std::fs::write(skill_path, skill)?;
    assert_validator_succeeds(&plugin_root)
}

#[test]
fn validator_rejects_nonroot_child_thread_creation_in_orchestration() -> TestResult {
    for instruction in [
        "A Sentinel acting as orchestrator MUST create a child thread.",
        "A Sentinel MUST create a child thread while the root orchestrator waits.",
        "A helper MUST create a child thread; the root orchestrator records the result.",
        "A Sentinel MUST create a child thread for the root orchestrator.",
        "A Sentinel working for the root orchestrator MUST create a child thread.",
        "A Sentinel and the root orchestrator MUST create a child thread.",
        "A child owner MUST create a child thread.",
        "The root orchestrator MUST ask a reviewer to create a child thread.",
        "The root orchestrator MUST request that a reviewer create a child thread.",
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = fixture(&temp)?;
        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let mut skill = std::fs::read_to_string(&skill_path)?;
        skill.push_str(&format!("\n{instruction}\n"));
        std::fs::write(skill_path, skill)?;
        assert_recursion_rejected(validator(&plugin_root)?, instruction);
    }
    Ok(())
}

#[test]
fn validator_rejects_nonroot_permission_in_orchestration() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = fixture(&temp)?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str("\nA child owner MAY spawn another helper.\n");
    std::fs::write(skill_path, skill)?;
    assert_recursion_rejected(
        validator(&plugin_root)?,
        "A child owner MAY spawn another helper.",
    );
    Ok(())
}

#[test]
fn validator_rejects_conjoined_nonroot_child_thread_creation() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = fixture(&temp)?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let mut skill = std::fs::read_to_string(&skill_path)?;
    skill.push_str(
        "\nThe root orchestrator MUST create a child thread and a Sentinel MUST create a child thread.\n",
    );
    std::fs::write(skill_path, skill)?;
    assert_recursion_rejected(
        validator(&plugin_root)?,
        "a Sentinel MUST create a child thread",
    );
    Ok(())
}

fn fixture(temp: &tempfile::TempDir) -> TestResult<std::path::PathBuf> {
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
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

fn assert_validator_succeeds(plugin_root: &Path) -> TestResult {
    let output = validator(plugin_root)?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

fn assert_recursion_rejected(output: Output, example: &str) {
    assert!(!output.status.success(), "{example}");
    assert!(
        stderr(&output).contains("permits recursive delegation"),
        "{example}"
    );
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
