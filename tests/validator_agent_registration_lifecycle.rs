use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn orchestration_guidance_covers_registration_schema_and_fork_preflight()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill = std::fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/agent-registration.md"),
    )?;
    for required in [
        "$CODEX_HOME/agents/codexy",
        "--diagnose",
        "role-discovery",
        "tool-schema",
        "fresh task",
        "fork_turns=\"none\"",
        "positive bounded count",
        "MUST NOT manage `features.multi_agent_v2`",
    ] {
        assert!(
            skill.contains(required),
            "missing orchestration guidance: {required}"
        );
    }
    Ok(())
}

#[test]
fn validator_requires_packaged_registration_support_module()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    std::fs::remove_file(
        plugin_root.join("skills/codex-orchestration/scripts/agent_registration_support.py"),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", path(&plugin_root)?, "--check-roles"])
        .output()?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("agent_registration_support.py must exist"));
    Ok(())
}

#[test]
fn validator_rejects_legacy_cache_registration_catalog_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let catalog_path = plugin_root.join("agents/catalog.toml");
    let catalog = std::fs::read_to_string(&catalog_path)?
        .replace(
            "codex-home-standalone-agent-projection",
            "user-config-agents-config_file",
        )
        .replace(
            "managed-codexy-subdirectory",
            "not-required-agent-files-are-custom-agent-compatible",
        );
    std::fs::write(catalog_path, catalog)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", path(&plugin_root)?, "--check-roles"])
        .output()?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("codex-home-standalone-agent-projection"));
    Ok(())
}

#[test]
fn diagnostics_separate_discovery_schema_and_fork_contracts()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let install = run(&plugin_root, &codex_home, &[])?;
    assert!(install.status.success(), "stderr:\n{}", stderr(&install));
    std::fs::write(
        codex_home.join("config.toml"),
        "[features.multi_agent_v2]\ntool_namespace = \"agents\"\nhide_spawn_agent_metadata = false\n",
    )?;

    let output = run(&plugin_root, &codex_home, &["--diagnose"])?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("A role-discovery: PASS (12 marker-owned standalone agents)"));
    assert!(
        stdout.contains("B tool-schema: CONFIGURED (namespace=agents, agent_type-visible=true)")
    );
    assert!(stdout.contains("fresh-task schema observation is still required"));
    assert!(stdout.contains(
        "C fork-turns: explicit agent_type requires none or a positive integer; all is incompatible"
    ));
    Ok(())
}

#[test]
fn registration_does_not_write_bytecode_into_plugin_scripts()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let scripts_root = plugin_root.join("skills/codex-orchestration/scripts");
    let bytecode = scripts_root.join("__pycache__");
    if bytecode.exists() {
        std::fs::remove_dir_all(&bytecode)?;
    }

    let output = run(
        &plugin_root,
        &temp.path().join("home/.codex"),
        &["--dry-run", "--diagnose"],
    )?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(
        !bytecode.exists(),
        "registration created bytecode in {}",
        scripts_root.display()
    );
    Ok(())
}

#[test]
fn registration_refuses_unowned_discovery_file_without_partial_writes()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let agents_root = codex_home.join("agents/codexy");
    std::fs::create_dir_all(&agents_root)?;
    let sentinel = agents_root.join("codexy-sentinel.toml");
    let user_contents = "name = \"codexy-sentinel\"\ndescription = \"User role\"\n";
    std::fs::write(&sentinel, user_contents)?;

    let output = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--codex-home",
            path(&codex_home)?,
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("is not owned by Codexy"));
    assert_eq!(std::fs::read_to_string(sentinel)?, user_contents);
    assert_eq!(std::fs::read_dir(agents_root)?.count(), 1);
    Ok(())
}

#[test]
fn update_and_uninstall_touch_only_marker_owned_discovery_files()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let agents_root = codex_home.join("agents/codexy");

    let install = run(&plugin_root, &codex_home, &[])?;
    assert!(install.status.success(), "stderr:\n{}", stderr(&install));
    let sentinel = agents_root.join("codexy-sentinel.toml");
    std::fs::write(&sentinel, "# CODEXY MANAGED AGENT\nname = \"old\"\n")?;
    let stale = agents_root.join("codexy-retired.toml");
    std::fs::write(
        &stale,
        "# CODEXY MANAGED AGENT\nname = \"codexy-retired\"\n",
    )?;
    let user_file = agents_root.join("personal.toml");
    std::fs::write(&user_file, "name = \"personal\"\n")?;

    let update = run(&plugin_root, &codex_home, &[])?;
    assert!(update.status.success(), "stderr:\n{}", stderr(&update));
    assert!(std::fs::read_to_string(&sentinel)?.contains("name = \"codexy-sentinel\""));
    assert!(!stale.exists());
    assert_eq!(
        std::fs::read_to_string(&user_file)?,
        "name = \"personal\"\n"
    );

    std::fs::remove_file(plugin_root.join("agents/catalog.toml"))?;
    let uninstall = run(&plugin_root, &codex_home, &["--uninstall"])?;
    assert!(
        uninstall.status.success(),
        "stderr:\n{}",
        stderr(&uninstall)
    );
    assert!(!sentinel.exists());
    assert_eq!(std::fs::read_to_string(user_file)?, "name = \"personal\"\n");
    Ok(())
}

fn run(
    plugin_root: &std::path::Path,
    codex_home: &std::path::Path,
    extra: &[&str],
) -> std::io::Result<std::process::Output> {
    registration_script(plugin_root)
        .args([
            "--plugin-root",
            plugin_root.to_str().unwrap(),
            "--codex-home",
            codex_home.to_str().unwrap(),
        ])
        .args(extra)
        .output()
}

fn installed_fixture(root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = root.join("installed-codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

fn registration_script(plugin_root: &std::path::Path) -> Command {
    Command::new(plugin_root.join("skills/codex-orchestration/scripts/register-codexy-agents"))
}

fn path(path: &std::path::Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
