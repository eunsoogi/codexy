use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_check_roles_rejects_agent_modal_policy_violations() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    assert!(sentinel.contains("MUST act as"));
    std::fs::write(&sentinel_path, sentinel.replace("MUST act as", "Run as"))?;

    let output = validator(&plugin_root, "--check-roles")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

#[test]
fn validator_cli_check_roles_rejects_forbidden_actions_without_must_not() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    assert!(sentinel.contains("Forbidden actions: MUST NOT edit files"));
    std::fs::write(
        &sentinel_path,
        sentinel.replace(
            "Forbidden actions: MUST NOT edit files",
            "Forbidden actions: edit files",
        ),
    )?;

    let output = validator(&plugin_root, "--check-roles")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("prohibitions must use MUST NOT"));
    Ok(())
}

#[test]
fn validator_cli_check_roles_rejects_openai_yaml_modal_policy_violations() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let prompt = std::fs::read_to_string(&prompt_path)?;
    assert!(prompt.contains("You MUST run $task-classification"));
    std::fs::write(
        &prompt_path,
        prompt.replace(
            "You MUST run $task-classification",
            "Run $task-classification",
        ),
    )?;

    let output = validator(&plugin_root, "--check-roles")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

#[test]
fn validator_cli_check_roles_rejects_skill_openai_yaml_modal_policy_violations() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let prompt_path = plugin_root.join("skills/git-workflow/agents/openai.yaml");
    let prompt = std::fs::read_to_string(&prompt_path)?;
    assert!(prompt.contains("You MUST use $git-workflow"));
    std::fs::write(
        &prompt_path,
        prompt.replace("You MUST use $git-workflow", "Run git workflow"),
    )?;

    let output = validator(&plugin_root, "--check-roles")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

fn validator(plugin_root: &Path, mode: &str) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            mode,
        ])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
