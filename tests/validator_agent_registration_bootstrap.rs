use std::path::Path;
use std::process::Command;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn installed_bootstrap_registers_agents_and_then_becomes_idempotent() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let codex_home = temp.path().join("home/.codex");
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");

    let first = Command::new(&bootstrap)
        .args(["--codex-home", path(&codex_home)?])
        .output()?;
    assert!(first.status.success(), "stderr:\n{}", stderr(&first));
    let first_stdout = stdout(&first);
    assert!(first_stdout.contains("A role-discovery: FAIL (0/12"));
    assert!(first_stdout.contains("A role-discovery: PASS (12 marker-owned"));
    assert!(first_stdout.contains("D bootstrap: RESTART_REQUIRED"));
    assert!(
        codex_home
            .join("agents/codexy/codexy-sentinel.toml")
            .is_file()
    );

    let second = Command::new(&bootstrap)
        .args(["--codex-home", path(&codex_home)?])
        .output()?;
    assert!(second.status.success(), "stderr:\n{}", stderr(&second));
    assert!(stdout(&second).contains("D bootstrap: READY"));
    assert!(!stdout(&second).contains("RESTART_REQUIRED"));
    Ok(())
}

#[test]
fn installed_bootstrap_rejects_plugin_root_overrides() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");
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
fn orchestration_guidance_bootstraps_exact_roles_without_generic_fallback() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;
    let registration = std::fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/agent-registration.md"),
    )?;

    for required in [
        "bootstrap-codexy-agents",
        "installed plugin",
        "RESTART_REQUIRED",
        "fresh task",
        "MUST NOT substitute",
        "`default`",
        "`worker`",
        "`explorer`",
    ] {
        assert!(
            skill.contains(required) || registration.contains(required),
            "missing bootstrap contract: {required}"
        );
    }
    assert!(!skill.contains("fall back to packaged TOML/catalog context"));
    Ok(())
}

#[test]
fn validator_requires_the_installed_bootstrap_entrypoint() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let hooks = std::fs::read_to_string(root.join("plugins/codexy/hooks/hooks.json"))?;
    assert!(!hooks.contains("bootstrap-codexy-agents"));
    assert!(!hooks.contains("register-codexy-agents"));
    Ok(())
}

fn path(path: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
