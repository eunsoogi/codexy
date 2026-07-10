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
