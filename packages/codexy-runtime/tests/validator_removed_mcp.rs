use std::process::Command;

use crate::support;

#[test]
fn validator_cli_rejects_removed_packaged_mcp_names_and_endpoints()
-> Result<(), Box<dyn std::error::Error>> {
    for (name, entry, expected) in [
        (
            "grep_app",
            serde_json::json!({"command": "grep_app"}),
            "disallowed MCP server",
        ),
        (
            "public-search",
            serde_json::json!({"url": "https://mcp.grep.app"}),
            "disallowed MCP value fragment",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_fixture(&plugin_root, &[".mcp.json"])?;
        let path = plugin_root.join(".mcp.json");
        let mut config: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        config[name] = entry;
        std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;

        let output = validate(&plugin_root, "--check-mcp")?;
        assert!(!output.status.success(), "removed MCP {name} was accepted");
        assert!(stderr(&output).contains(expected), "stderr: {}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_removed_custom_agent_mcp_references()
-> Result<(), Box<dyn std::error::Error>> {
    for (fragment, expected) in [
        (
            "\n[mcp_servers.grep_app]\ncommand = \"example\"\n",
            "removed MCP server",
        ),
        (
            "\n[mcp_servers.public_search]\ncommand = \"grep_app\"\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\nurl = \"https://mcp.grep.app\"\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\ncommand = \"/usr/local/bin/grep_app\"\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\nurl = \"https://grep.app/mcp\"\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\nurl = \"https://grep.app:invalid/mcp\"\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\ncommand = 'C:\\tools\\grep_app.exe'\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\ncommand = 'C:\\tools\\grep_app.ExE'\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\nurl = \"https://user@grep.app:443/mcp\"\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\nurl = 'https://grep.app\\mcp'\n",
            "references removed MCP endpoint or command",
        ),
        (
            "\n[mcp_servers.public_search]\nurl = 'https://grep.app\\@safe.example/mcp'\n",
            "references removed MCP endpoint or command",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_fixture(&plugin_root, &["agents/codexy-architect.toml"])?;
        let path = plugin_root.join("agents/codexy-architect.toml");
        let mut agent = std::fs::read_to_string(&path)?;
        agent.push_str(fragment);
        std::fs::write(&path, agent)?;

        let output = validate(&plugin_root, "--check-roles")?;
        assert!(!output.status.success(), "removed agent MCP was accepted");
        assert!(stderr(&output).contains(expected), "stderr: {}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_cli_preserves_unrelated_mcp_command_and_url_identities()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root, &[".mcp.json", "agents/codexy-architect.toml"])?;
    let agent_path = plugin_root.join("agents/codexy-architect.toml");
    std::fs::write(
        &agent_path,
        format!(
            "{}\n[mcp_servers.unrelated_command]\ncommand = \"/usr/local/bin/grep\"\n\n[mcp_servers.unrelated_windows]\ncommand = 'C:\\tools\\grep.exe'\n\n[mcp_servers.unrelated_url]\nurl = \"https://grep.app.example/mcp\"\n\n[mcp_servers.safe_query]\nurl = \"https://example.com/path?next=@grep.app\"\n\n[mcp_servers.safe_userinfo]\nurl = \"https://grep.app@safe.example/mcp\"\n\n[mcp_servers.safe_malformed_boundary]\nurl = 'https://safe.example\\@grep.app/mcp'\n",
            std::fs::read_to_string(&agent_path)?
        ),
    )?;

    assert!(validate(&plugin_root, "--check-mcp")?.status.success());
    assert!(validate(&plugin_root, "--check-roles")?.status.success());
    Ok(())
}

fn copy_fixture(
    plugin_root: &std::path::Path,
    mutable_files: &[&str],
) -> std::io::Result<()> {
    let repository = codexy_runtime::paths::repository_root();
    support::copy_dir(&repository.join("plugins/codexy"), plugin_root)?;
    for relative in mutable_files {
        let relative = std::path::Path::new(relative);
        let devtools = repository.join("plugins/codexy-devtools").join(relative);
        let source = if devtools.is_file() {
            devtools
        } else {
            repository.join("plugins/codexy").join(relative)
        };
        let target = plugin_root.join(relative);
        std::fs::create_dir_all(target.parent().expect("fixture parent"))?;
        std::fs::copy(source, target)?;
    }
    Ok(())
}

fn validate(
    plugin_root: &std::path::Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            mode,
        ])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
