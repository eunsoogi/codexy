use std::path::Path;
use crate::support::FixtureCommand as Command;

#[path = "structured_contract_artifacts.rs"]
mod structured_contract_artifacts;
use crate::support;

use structured_contract_artifacts::TextShape;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn installed_bootstrap_registers_agents_and_then_becomes_idempotent() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path(), &[])?;
    let codex_home = temp.path().join("home/.codex");
    let bootstrap = plugin_root.join("skills/orchestration/scripts/bootstrap-codexy-agents");

    let first = Command::new(&bootstrap)
        .args(["--codex-home", path(&codex_home)?])
        .output()?;
    assert!(first.status.success(), "stderr:\n{}", stderr(&first));
    let first_stdout = stdout(&first);
    support::assert_structured_literals(
        &first_stdout,
        "agent registration bootstrap transition",
        &[
            "A role-discovery: FAIL (0/7",
            "A role-discovery: PASS (7 marker-owned",
            "D bootstrap: RESTART_REQUIRED",
        ],
    );
    assert!(
        codex_home
            .join("agents/codexy/codexy-sentinel.toml")
            .is_file()
    );

    let second = Command::new(&bootstrap)
        .args(["--codex-home", path(&codex_home)?])
        .output()?;
    assert!(second.status.success(), "stderr:\n{}", stderr(&second));
    let second_stdout = stdout(&second);
    support::assert_structured_literals(
        &second_stdout,
        "idempotent agent bootstrap state",
        &["D bootstrap: READY"],
    );
    TextShape::new(&second_stdout)
        .assert_absent_concepts("idempotent bootstrap restart state", &["restart_required"]);
    Ok(())
}

#[test]
fn installed_bootstrap_rejects_plugin_root_overrides() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path(), &[])?;
    let bootstrap = plugin_root.join("skills/orchestration/scripts/bootstrap-codexy-agents");
    let codex_home = temp.path().join("home/.codex");

    let output = Command::new(&bootstrap)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--codex-home",
            path(&codex_home)?,
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "bootstrap accepted --plugin-root override"
    );
    assert!(stderr(&output).contains("must resolve agents from its installed package"));
    assert!(!codex_home.exists(), "rejected override mutated CODEX_HOME");

    let inline_override = format!("--plugin-root={}", path(&plugin_root)?);
    let inline = Command::new(&bootstrap)
        .args([inline_override.as_str(), "--codex-home", path(&codex_home)?])
        .output()?;
    assert!(
        !inline.status.success(),
        "bootstrap accepted inline --plugin-root override"
    );
    assert!(stderr(&inline).contains("must resolve agents from its installed package"));
    assert!(!codex_home.exists(), "rejected override mutated CODEX_HOME");
    Ok(())
}

#[test]
fn pre_start_bootstrap_rejects_custom_config_discovery_roots() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path(), &[])?;
    let custom_root = temp.path().join("custom-profile");
    let config = custom_root.join("config.toml");
    let output = Command::new(plugin_root.join("bootstrap-codexy-agents"))
        .args(["--config", path(&config)?])
        .output()?;

    assert!(!output.status.success(), "bootstrap accepted --config");
    assert!(stderr(&output).contains("does not support --config"));
    assert!(
        !custom_root.exists(),
        "rejected custom config mutated its discovery root"
    );
    Ok(())
}

#[test]
fn validator_requires_the_installed_bootstrap_entrypoint() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(
        temp.path(),
        &[Path::new(
            "skills/orchestration/scripts/bootstrap-codexy-agents",
        )],
    )?;
    let bootstrap = plugin_root.join("skills/orchestration/scripts/bootstrap-codexy-agents");
    if bootstrap.exists() {
        std::fs::remove_file(&bootstrap)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", path(&plugin_root)?, "--check-roles"])
        .output()?;
    assert!(
        !output.status.success(),
        "validator accepted missing bootstrap"
    );
    assert!(stderr(&output).contains("bootstrap-codexy-agents must exist"));
    Ok(())
}

#[test]
fn lifecycle_hooks_do_not_run_the_registration_bootstrap() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let hooks = std::fs::read_to_string(root.join("plugins/codexy/hooks/hooks.json"))?;
    TextShape::new(&hooks).assert_absent_concepts(
        "registration lifecycle hook boundary",
        &["bootstrap-codexy-agents", "register_codexy_agents.py"],
    );
    Ok(())
}

fn path(path: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}

fn installed_plugin(root: &Path, mutable_files: &[&Path]) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = root.join("installed-codexy");
    support::copy_plugin_fixture_into_with_mutable_files(&plugin_root, mutable_files)?;
    Ok(plugin_root)
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
