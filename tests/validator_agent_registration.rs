use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn register_codexy_agents_writes_config_file_entries() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let config_path = temp.path().join("home/.codex/config.toml");

    let output = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--config",
            path(&config_path)?,
        ])
        .output()?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let config = std::fs::read_to_string(&config_path)?;
    let parsed: toml::Value = toml::from_str(&config)?;
    assert_eq!(
        parsed["agents"]["reviewer"]["config_file"].as_str(),
        Some(path(
            &plugin_root.join("agents/reviewer.toml").canonicalize()?
        )?)
    );
    assert_eq!(
        parsed["agents"]["planner"]["config_file"].as_str(),
        Some(path(
            &plugin_root.join("agents/planner.toml").canonicalize()?
        )?)
    );
    Ok(())
}

#[test]
fn register_codexy_agents_dry_run_does_not_touch_config() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let config_path = temp.path().join("home/.codex/config.toml");

    let output = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--config",
            path(&config_path)?,
            "--dry-run",
        ])
        .output()?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(!config_path.exists());
    Ok(())
}

#[test]
fn register_codexy_agents_refuses_unmanaged_conflicts() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let config_path = temp.path().join("home/.codex/config.toml");
    std::fs::create_dir_all(config_path.parent().ok_or("config parent")?)?;
    std::fs::write(
        &config_path,
        "[agents.reviewer]\ndescription = \"Existing reviewer\"\n",
    )?;

    let output = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--config",
            path(&config_path)?,
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("already defines unmanaged Codex agent"));
    Ok(())
}

#[test]
fn register_codexy_agents_uninstall_removes_only_managed_block()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let config_path = temp.path().join("home/.codex/config.toml");
    std::fs::create_dir_all(config_path.parent().ok_or("config parent")?)?;
    std::fs::write(&config_path, "model = \"gpt-5.5\"\n")?;

    let install = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--config",
            path(&config_path)?,
        ])
        .output()?;
    assert!(install.status.success(), "stderr:\n{}", stderr(&install));

    let uninstall = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--config",
            path(&config_path)?,
            "--uninstall",
        ])
        .output()?;
    assert!(
        uninstall.status.success(),
        "stderr:\n{}",
        stderr(&uninstall)
    );
    let config = std::fs::read_to_string(&config_path)?;
    assert!(config.contains("model = \"gpt-5.5\""));
    assert!(!config.contains("BEGIN CODEXY MANAGED AGENTS"));
    assert!(!config.contains("[agents.reviewer]"));
    Ok(())
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
