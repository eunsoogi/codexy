use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn register_codexy_agents_migrates_cache_config_to_stable_discovery_files()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let old_plugin_root = temp.path().join("cache/codexy/1.0.0");
    let plugin_root = installed_fixture(&temp.path().join("cache/codexy/1.0.1"))?;
    let codex_home = temp.path().join("home/.codex");
    let config_path = codex_home.join("config.toml");
    std::fs::create_dir_all(config_path.parent().ok_or("config parent")?)?;
    std::fs::write(
        &config_path,
        format!(
            "model = \"gpt-5.5\"\n\n# BEGIN CODEXY MANAGED AGENTS\n[agents.codexy-sentinel]\nconfig_file = {:?}\n# END CODEXY MANAGED AGENTS\n",
            old_plugin_root.join("agents/codexy-sentinel.toml")
        ),
    )?;

    let output = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--codex-home",
            path(&codex_home)?,
        ])
        .output()?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let config = std::fs::read_to_string(&config_path)?;
    assert!(config.contains("model = \"gpt-5.5\""));
    assert!(!config.contains("BEGIN CODEXY MANAGED AGENTS"));
    assert!(!config.contains("cache/codexy"));

    let installed = codex_home.join("agents/codexy/codexy-sentinel.toml");
    let installed_text = std::fs::read_to_string(installed)?;
    assert!(installed_text.starts_with("# CODEXY MANAGED AGENT\n"));
    assert!(installed_text.contains("name = \"codexy-sentinel\""));
    Ok(())
}

#[test]
fn register_codexy_agents_writes_stable_discovery_files() -> Result<(), Box<dyn std::error::Error>>
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
        ])
        .output()?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(!config_path.exists());
    let agents_root = config_path
        .parent()
        .ok_or("config parent")?
        .join("agents/codexy");
    for name in [
        "codexy-architect",
        "codexy-tracer",
        "codexy-scribe",
        "codexy-cartographer",
        "codexy-forge",
        "codexy-weaver",
        "codexy-pathfinder",
        "codexy-auditor",
        "codexy-sculptor",
        "codexy-shipwright",
        "codexy-sentinel",
        "codexy-warden",
    ] {
        let installed = std::fs::read_to_string(agents_root.join(format!("{name}.toml")))?;
        assert!(installed.starts_with("# CODEXY MANAGED AGENT\n"));
        assert!(installed.contains(&format!("name = \"{name}\"")));
    }
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
fn register_codexy_agents_does_not_require_tomli_fallback() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let config_path = temp.path().join("home/.codex/config.toml");
    let fake_modules = temp.path().join("fake-pythonpath");
    std::fs::create_dir(&fake_modules)?;
    std::fs::write(
        fake_modules.join("tomllib.py"),
        "raise ModuleNotFoundError('simulated Python 3.10')\n",
    )?;
    std::fs::write(
        fake_modules.join("tomli.py"),
        "raise RuntimeError('unbundled tomli was imported')\n",
    )?;

    let output = registration_script(&plugin_root)
        .env("PYTHONPATH", path(&fake_modules)?)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--config",
            path(&config_path)?,
        ])
        .output()?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(
        std::fs::read_to_string(
            config_path
                .parent()
                .ok_or("config parent")?
                .join("agents/codexy/codexy-sentinel.toml")
        )?
        .contains("name = \"codexy-sentinel\"")
    );
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
        "[agents.codexy-sentinel]\ndescription = \"Existing reviewer\"\n",
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
    assert!(!config.contains("[agents.codexy-sentinel]"));
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
