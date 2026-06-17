use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn register_codexy_agents_refuses_quoted_unmanaged_conflicts()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let config_path = temp.path().join("home/.codex/config.toml");
    write_config(&config_path, "[agents.\"reviewer\"]\ndescription = \"Existing reviewer\"\n")?;

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
fn register_codexy_agents_backup_uses_python310_compatible_timestamp()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let config_path = temp.path().join("home/.codex/config.toml");
    write_config(&config_path, "model = \"gpt-5.5\"\n")?;

    let output = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--config",
            path(&config_path)?,
        ])
        .output()?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(!script_text(&plugin_root)?.contains("datetime.UTC"));
    assert_eq!(backup_count(config_path.parent().ok_or("config parent")?)?, 1);
    Ok(())
}

#[test]
fn register_codexy_agents_uninstall_does_not_require_valid_catalog()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let config_path = temp.path().join("home/.codex/config.toml");
    write_config(
        &config_path,
        "model = \"gpt-5.5\"\n\n# BEGIN CODEXY MANAGED AGENTS\n[agents.reviewer]\nconfig_file = \"stale\"\n# END CODEXY MANAGED AGENTS\n",
    )?;
    std::fs::remove_file(plugin_root.join("agents/catalog.toml"))?;

    let output = registration_script(&plugin_root)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--config",
            path(&config_path)?,
            "--uninstall",
        ])
        .output()?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let config = std::fs::read_to_string(config_path)?;
    assert!(config.contains("model = \"gpt-5.5\""));
    assert!(!config.contains("BEGIN CODEXY MANAGED AGENTS"));
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

fn write_config(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(path.parent().expect("config parent"))?;
    std::fs::write(path, contents)
}

fn registration_script(plugin_root: &std::path::Path) -> Command {
    Command::new(plugin_root.join("skills/codex-orchestration/scripts/register-codexy-agents"))
}

fn script_text(plugin_root: &std::path::Path) -> std::io::Result<String> {
    std::fs::read_to_string(
        plugin_root.join("skills/codex-orchestration/scripts/register-codexy-agents"),
    )
}

fn backup_count(config_dir: &std::path::Path) -> std::io::Result<usize> {
    Ok(std::fs::read_dir(config_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("config.toml.codexy-backup-")
        })
        .count())
}

fn path(path: &std::path::Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
