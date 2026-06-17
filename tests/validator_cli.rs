#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

#[test]
fn validator_cli_checks_all_contract_surfaces() -> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "--check",
        "--check-mcp",
        "--check-hooks",
        "--check-lsp",
        "--check-roles",
        "--print-covered-extensions",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .arg(mode)
            .output()?;
        assert!(
            output.status.success(),
            "validator {mode} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_mixed_type_string_arrays() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let mcp_path = plugin_root.join(".mcp.json");
    let mut mcp_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mcp_path)?)?;
    mcp_config["lsp"]["args"] = serde_json::json!(["run", 7, "--quiet"]);
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_config)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-mcp",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject mixed-type args arrays"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("args must be an array of strings"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_mcp_entrypoints_outside_plugin_root()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    std::fs::write(temp.path().join("outside.txt"), "outside\n")?;
    let mcp_path = plugin_root.join(".mcp.json");
    let mut mcp_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mcp_path)?)?;
    mcp_config["lsp"]["command"] = serde_json::json!("sh");
    mcp_config["lsp"]["args"] = serde_json::json!(["./../outside.txt"]);
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_config)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-mcp",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject MCP entrypoints outside the plugin root"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must stay inside the plugin root"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_script_runtime_mcp_entrypoints() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let script_name = ["server", &["j", "s"].join("")].join(".");
    let script_path = plugin_root.join("mcp").join(&script_name);
    std::fs::write(&script_path, "removed runtime\n")?;
    let mcp_path = plugin_root.join(".mcp.json");
    let mut mcp_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mcp_path)?)?;
    let command_name = ["no", "de"].join("");
    mcp_config["lsp"]["command"] = serde_json::json!(command_name);
    mcp_config["lsp"]["args"] = serde_json::json!([format!("./mcp/{script_name}"), "--stdio"]);
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_config)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-mcp",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject script runtime MCP entrypoints"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must not use"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_installed_plugin_mcp_entrypoints() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;

    let mcp_path = plugin_root.join(".mcp.json");
    let mcp_config: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&mcp_path)?)?;
    for server_name in ["lsp", "codegraph"] {
        let command = mcp_config[server_name]["command"]
            .as_str()
            .ok_or("MCP command must be a string")?;
        assert!(
            command.starts_with("./"),
            "{server_name} command must be plugin-relative for installed packages"
        );
        assert!(
            plugin_root.join(command).is_file(),
            "{server_name} command must exist inside the installed plugin"
        );
        #[cfg(unix)]
        assert!(
            plugin_root.join(command).metadata()?.permissions().mode() & 0o111 != 0,
            "{server_name} command must be executable inside the installed plugin"
        );
        assert!(
            !mcp_config[server_name]["args"]
                .as_array()
                .ok_or("MCP args must be an array")?
                .iter()
                .any(|arg| arg.as_str().is_some_and(|item| item.contains("../"))),
            "{server_name} args must not escape the installed plugin"
        );
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-mcp",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "validator should accept installed plugin-local MCP entrypoints\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_installed_plugin_hook_entrypoints()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;

    let hooks_path = plugin_root.join("hooks/hooks.json");
    let hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    let command = hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["command"]
        .as_str()
        .ok_or("SessionStart hook command must be a string")?;
    assert!(
        command.contains("${PLUGIN_ROOT}/hooks/codexy-routing-context.sh"),
        "hook command must resolve through PLUGIN_ROOT for installed packages"
    );
    assert!(
        !command.contains("PLUGIN_DATA") && !command.contains("~/.codex"),
        "hook command must not reference writable plugin data or user state"
    );
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    assert!(
        script_path.is_file(),
        "hook command target must exist inside the installed plugin"
    );
    #[cfg(unix)]
    assert!(
        script_path.metadata()?.permissions().mode() & 0o111 != 0,
        "hook command target must be executable inside the installed plugin"
    );

    let hook_output = Command::new(&script_path).arg("SessionStart").output()?;
    assert!(
        hook_output.status.success(),
        "hook script should emit context successfully\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&hook_output.stdout),
        String::from_utf8_lossy(&hook_output.stderr)
    );
    let hook_json: serde_json::Value = serde_json::from_slice(&hook_output.stdout)?;
    assert_eq!(
        hook_json["hookSpecificOutput"]["hookEventName"],
        "SessionStart"
    );
    assert!(
        hook_json["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .is_some_and(|context| context.contains("$codex-orchestration")),
        "hook output should surface lightweight Codexy routing context"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "validator should accept installed plugin-local hook entrypoints\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_plugin_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    std::fs::remove_file(plugin_root.join("hooks/hooks.json"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject missing plugin hook config"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hooks/hooks.json"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_hooks_without_plugin_root_command()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["command"] =
        serde_json::json!("./hooks/codexy-routing-context.sh SessionStart");
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject hook commands that do not resolve through PLUGIN_ROOT"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("must reference a packaged ${PLUGIN_ROOT} path"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_hook_user_state_mutation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    let mut script = std::fs::read_to_string(&script_path)?;
    script.push_str("\ntouch ~/.codex/codexy-hook-state\n");
    std::fs::write(&script_path, script)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject hook scripts that mutate user Codex state"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hook script must not contain"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_non_boolean_hook_async() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["async"] = serde_json::json!("false");
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-hooks",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject non-boolean hook async values"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hook async must be a boolean"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_empty_agent_list_entries() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let planner_path = plugin_root.join("agents/planner.toml");
    let mut planner = std::fs::read_to_string(&planner_path)?;
    planner = planner.replace("inputs = [", "inputs = [\"\", ");
    std::fs::write(&planner_path, planner)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject empty agent list entries"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("inputs must be a list of non-empty strings"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_manifest_prompt_without_orchestration_route()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["interface"]["defaultPrompt"] = serde_json::json!([
        "Use Codexy as orchestrator; split the request into atomic issue-sized lanes before editing.",
        "Track goals and todos; assign specialist roles with multi-agent or multi-thread work.",
        "Verify evidence, require Codex review, and use squash-merge gates before completion."
    ]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject manifest prompt without $codex-orchestration"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("interface.defaultPrompt must route through $codex-orchestration"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_top_level_prompt_without_orchestration_route()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let mut prompt = std::fs::read_to_string(&prompt_path)?;
    prompt = prompt.replace("$codex-orchestration", "Codexy orchestration");
    std::fs::write(&prompt_path, prompt)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject top-level prompt without $codex-orchestration"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("interface.default_prompt must route through $codex-orchestration"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_top_level_prompt_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    std::fs::remove_file(plugin_root.join("agents/openai.yaml"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject missing top-level agents/openai.yaml"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("agents/openai.yaml is required for plugin invocation metadata"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_skill_prompt_without_orchestration_route()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let prompt_path = plugin_root.join("skills/git-workflow/agents/openai.yaml");
    let prompt = std::fs::read_to_string(&prompt_path)?;
    assert!(
        !prompt.contains("$codex-orchestration"),
        "fixture should prove non-orchestration skill prompts remain valid"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "validator should allow non-orchestration skill prompts\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_tab_indented_prompt_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let mut prompt = std::fs::read_to_string(&prompt_path)?;
    prompt = prompt.replace("  display_name:", "\tdisplay_name:");
    std::fs::write(&prompt_path, prompt)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject tab-indented prompt YAML"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must not contain tab indentation"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_mixed_space_tab_prompt_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let mut prompt = std::fs::read_to_string(&prompt_path)?;
    prompt = prompt.replace("  display_name:", " \tdisplay_name:");
    std::fs::write(&prompt_path, prompt)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject mixed space-tab prompt YAML indentation"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must not contain tab indentation"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn copy_dir(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            if entry.file_name() == "target" {
                continue;
            }
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

#[test]
fn sync_version_cli_checks_manifest_marketplace_parity() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .arg("--check")
        .output()?;
    assert!(
        output.status.success(),
        "sync-version --check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("plugin version sync ok"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}
