use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_line_wrapped_recursive_permissions() -> TestResult {
    for permission in [
        "Allowed actions:\n- spawning another helper for QA.",
        "Allowed actions:\n1. Spawning another helper for QA.",
        "A helper MAY\nspawn another helper.",
        "A helper is allowed to spawn\nanother helper.",
    ] {
        assert_recursive_role_permission_rejected(permission)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_recursive_actions_after_unrelated_negations() -> TestResult {
    for permission in [
        "MUST spawn validator_edge_pass and workflow_ownership_pass as additional helpers.",
        "MUST create a child thread.",
        "A Sentinel acting as orchestrator MUST create a child thread.",
        "A Sentinel MUST create a child thread while the root orchestrator waits.",
        "A helper MUST create a child thread; the root orchestrator records the result.",
        "A Sentinel MUST create a child thread for the root orchestrator.",
        "MUST immediately spawn another helper.",
        "MUST spawn another Sentinel.",
        "MUST delegate work to another specialist.",
        "Allowed actions: creating reviewer tasks.",
        "Allowed actions: delegating work to helper threads.",
        "MUST NOT merge, but MAY spawn another helper.",
        "A helper MAY not edit files, but MAY spawn another helper.",
        "Allowed actions: MUST NOT edit files, but spawn another helper.",
        "Permitted actions: MUST NOT merge, but create reviewer tasks.",
        "A helper is not allowed to merge, but is allowed to spawn another helper.",
    ] {
        assert_recursive_role_permission_rejected(permission)?;
    }
    Ok(())
}

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
fn validator_does_not_flag_punctuated_nonrecursive_prohibitions() -> TestResult {
    for prohibition in [
        "A helper MAY, under no circumstances, spawn another helper.",
        "A helper MAY not, under any circumstances, spawn another helper.",
        "A helper MAY never, even during recovery, create another reviewer task.",
        "A helper is not allowed, under this contract, to delegate work to another reviewer.",
        "Allowed actions: map files, but MUST NOT spawn another helper.",
        "Every helper MUST NOT spawn, delegate to, or create any additional agent.",
    ] {
        assert_role_recursion_not_reported(prohibition)?;
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
    std::fs::write(skill_path, skill)?;
    assert_validator_succeeds(&plugin_root)?;
    Ok(())
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

#[test]
fn validator_rejects_qualified_allowed_recursive_delegation() -> TestResult {
    assert_recursive_role_permission_rejected(
        "A helper is allowed, after owner approval, to spawn another helper.",
    )
}

fn assert_recursive_role_permission_rejected(permission: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = fixture(&temp)?;
    let role_path = plugin_root.join("agents/codexy-cartographer.toml");
    let role = std::fs::read_to_string(&role_path)?;
    std::fs::write(
        &role_path,
        role.replacen("\n\"\"\"", &format!("\n{permission}\n\"\"\""), 1),
    )?;
    assert_recursion_rejected(validator(&plugin_root)?, permission);
    Ok(())
}

fn assert_role_recursion_not_reported(prohibition: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = fixture(&temp)?;
    let role_path = plugin_root.join("agents/codexy-cartographer.toml");
    let role = std::fs::read_to_string(&role_path)?;
    std::fs::write(
        &role_path,
        role.replacen("\n\"\"\"", &format!("\n{prohibition}\n\"\"\""), 1),
    )?;
    assert_recursion_not_reported(&plugin_root)
}

fn assert_recursion_not_reported(plugin_root: &Path) -> TestResult {
    assert!(!stderr(&validator(plugin_root)?).contains("permits recursive delegation"));
    Ok(())
}

fn assert_validator_succeeds(plugin_root: &Path) -> TestResult {
    let output = validator(plugin_root)?;
    assert!(output.status.success(), "{}", stderr(&output));
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
