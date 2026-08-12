use crate::support::FixtureCommand as Command;

use crate::support;

use std::path::Path;

const MUTABLE_PLUGIN_FILES: &[&str] = &[
    "agents/catalog.toml",
    "agents/codexy-sentinel.toml",
];

#[test]
fn conflict_detector_covers_quoted_dotted_and_inline_forms()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = plugin_fixture()?;
    let scripts = fixture
        .root()
        .join("skills/orchestration/scripts");
    let script_path = path(&scripts)?;
    let body = r#"
import sys
sys.path.insert(0, sys.argv[1])
from agent_registration_support import find_conflicts

names = {"codexy-sentinel", "codexy-cartographer"}
cases = [
    (r'''[agents."codexy-sentinel"]\ndescription = "Existing reviewer"\n''', {"codexy-sentinel"}),
    (r'''[agents.codexy-sentinel.mcp_servers.local]\ncommand = "local"\n''', {"codexy-sentinel"}),
    (r'''[agents.'codexy-sentinel']\ndescription = 'Existing reviewer'\n''', {"codexy-sentinel"}),
    (r'''["agents"."codexy-sentinel"]\nconfig_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''["agents"."codexy-sentinel".mcp_servers.local]\ncommand = "local"\n''', {"codexy-sentinel"}),
    (r'''["ag\u0065nts"."codexy-sentinel"]\nconfig_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''["\U00000061gents"."codexy-sentinel"]\nconfig_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''['agents'.'codexy-sentinel']\nconfig_file = 'existing.toml'\n''', {"codexy-sentinel"}),
    (r'''["agents".codexy-cartographer] # local explorer\nconfig_file = "existing.toml"\n''', {"codexy-cartographer"}),
    (r'''[agents.codexy-sentinel] # local reviewer\nconfig_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''[agents."codexy-sentinel"] # local reviewer\nconfig_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''[agents.'codexy-sentinel'] # local reviewer\nconfig_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''[agents."codexy\u002dsentinel"]\nconfig_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''agents.codexy-sentinel.config_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''agents.codexy-sentinel = { config_file = "existing.toml" }\n''', {"codexy-sentinel"}),
    (r'''agents . codexy-sentinel . config_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''agents.'codexy-sentinel'.config_file = 'existing.toml'\n''', {"codexy-sentinel"}),
    (r'''agents.'codexy-sentinel' = { config_file = 'existing.toml' }\n''', {"codexy-sentinel"}),
    (r'''agents . 'codexy-sentinel' . config_file = 'existing.toml'\n''', {"codexy-sentinel"}),
    (r'''"agents"."codexy-sentinel".config_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r"""'agents'.'codexy-sentinel'.config_file = 'existing.toml'\n""", {"codexy-sentinel"}),
    (r'''agents.codexy-cartographer.config_file = "existing.toml"\n''', {"codexy-cartographer"}),
    (r'''[agents]\n"codexy-sentinel".config_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''[agents] # local agents table\n"codexy-sentinel".config_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''["agents"]\n"codexy-sentinel".config_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''["ag\u0065nts"]\n"codexy-sentinel".config_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''["\U00000061gents"]\n"codexy-sentinel".config_file = "existing.toml"\n''', {"codexy-sentinel"}),
    (r'''[agents]\n'codexy-sentinel'.config_file = 'existing.toml'\n''', {"codexy-sentinel"}),
    (r'''['agents']\n'codexy-sentinel'.config_file = 'existing.toml'\n''', {"codexy-sentinel"}),
    (r'''[agents]\ncodexy-cartographer.config_file = "existing.toml"\n''', {"codexy-cartographer"}),
    (r'''[agents]\ncodexy-sentinel = { config_file = "existing.toml" }\n''', {"codexy-sentinel"}),
    (r'''[agents]\n"codexy-sentinel" = { config_file = "existing.toml" }\n''', {"codexy-sentinel"}),
    (r'''["agents"]\n'codexy-sentinel' = { config_file = 'existing.toml' }\n''', {"codexy-sentinel"}),
    (r'''agents = { max_threads = 6 }\n''', names),
    (r'''"agents" = { max_threads = 6 }\n''', names),
    (r"""'agents' = { max_threads = 6 }\n""", names),
    (r'''agents = { codexy-sentinel = { config_file = "existing.toml" } }\n''', names),
]
for encoded, expected in cases:
    existing = encoded.replace("\\n", "\n")
    found = find_conflicts(existing, names)
    assert found == expected, (existing, found, expected)
"#;
    let output = Command::new("python3")
        .args(["-c", body, script_path])
        .output()?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn register_codexy_agents_backup_uses_python310_compatible_timestamp()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = plugin_fixture()?;
    let plugin_root = fixture.root();
    let config_path = temp.path().join("home/.codex/config.toml");
    write_config(
        &config_path,
        "model = \"gpt-5.5\"\n\n# BEGIN CODEXY MANAGED AGENTS\n[agents.codexy-sentinel]\nconfig_file = \"stale\"\n# END CODEXY MANAGED AGENTS\n",
    )?;

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
    assert_eq!(
        backup_count(config_path.parent().ok_or("config parent")?)?,
        1
    );
    Ok(())
}

#[test]
fn register_codexy_agents_uninstall_does_not_require_valid_catalog()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = plugin_fixture()?;
    let plugin_root = fixture.root();
    let config_path = temp.path().join("home/.codex/config.toml");
    write_config(
        &config_path,
        "model = \"gpt-5.5\"\n\n# BEGIN CODEXY MANAGED AGENTS\n[agents.codexy-sentinel]\nconfig_file = \"stale\"\n# END CODEXY MANAGED AGENTS\n",
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

#[test]
fn register_codexy_agents_allows_supported_agent_config_tables()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = plugin_fixture()?;
    let plugin_root = fixture.root();
    let agent_path = plugin_root.join("agents/codexy-sentinel.toml");
    let mut agent = std::fs::read_to_string(&agent_path)?;
    agent.push_str(
        "\n[mcp_servers.example_mcp]\ncommand = \"example_mcp\"\n\n[[skills.config]]\nname = \"codexy:qa\"\n",
    );
    std::fs::write(agent_path, agent)?;
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("would install 8 Codexy agents"));
    Ok(())
}

fn plugin_fixture() -> std::io::Result<support::PluginFixture> {
    let mutable_files = MUTABLE_PLUGIN_FILES.iter().map(Path::new).collect::<Vec<_>>();
    support::plugin_fixture_with_mutable_files(&mutable_files)
}

fn write_config(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(path.parent().expect("config parent"))?;
    std::fs::write(path, contents)
}

fn registration_script(plugin_root: &std::path::Path) -> Command {
    Command::new(plugin_root.join("skills/orchestration/scripts/register-codexy-agents"))
}

fn script_text(plugin_root: &std::path::Path) -> std::io::Result<String> {
    std::fs::read_to_string(
        plugin_root.join("skills/orchestration/scripts/register-codexy-agents"),
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
