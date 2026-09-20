use serde_json::Value;

#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::{Command, Stdio};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MATCHER: &str = "^(?:mcp__[^ ]+__)?watcher_wait$";

#[cfg(unix)]
fn run_pretool_hook(
    root: &Path,
    cache: &Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let mut command = Command::new("/bin/sh");
    command
        .arg(root.join("plugins/codexy/hooks/codexy-watcher-interrupt.sh"))
        .arg("PreToolUse")
        .env_clear()
        .env("PLUGIN_ROOT", root.join("plugins/codexy"))
        .env("CODEXY_RUNTIME_CACHE_DIR", cache)
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        .env_remove("CODEXY_RUNTIME_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(&json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__codexy-watcher__watcher_wait",
            "tool_input": {"sessionId": "session", "parentToken": "token"}
        }))?)?;
    Ok(child.wait_with_output()?)
}
const SHELL: &str = "\"${PLUGIN_ROOT}/hooks/codexy-watcher-interrupt.sh\"";
const WINDOWS: &str = "\"${PLUGIN_ROOT}/hooks/codexy-watcher-interrupt.cmd\"";

#[test]
fn watcher_wait_interrupt_registration_is_explicit_and_synchronous() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let hooks: Value = serde_json::from_str(&std::fs::read_to_string(root.join("hooks.json"))?)?;
    let pretool = hooks["hooks"]["PreToolUse"]
        .as_array()
        .ok_or("PreToolUse groups")?
        .iter()
        .find(|group| group["matcher"] == MATCHER)
        .ok_or("watcher_wait PreToolUse group")?;
    let pretool_handler = &pretool["hooks"][0];
    assert_eq!(pretool_handler["command"], format!("{SHELL} PreToolUse"));
    assert_eq!(pretool_handler["commandWindows"], format!("{WINDOWS} PreToolUse"));
    assert_eq!(pretool_handler["timeout"], 5);

    let interrupt = hooks["hooks"]["Interrupt"].as_array().ok_or("Interrupt groups")?;
    assert_eq!(interrupt.len(), 1);
    assert!(interrupt[0].get("matcher").is_none());
    let interrupt_handler = &interrupt[0]["hooks"][0];
    assert_eq!(interrupt_handler["command"], format!("{SHELL} Interrupt"));
    assert_eq!(interrupt_handler["commandWindows"], format!("{WINDOWS} Interrupt"));
    assert_eq!(interrupt_handler["timeout"], 3);
    Ok(())
}

#[test]
fn watcher_wait_interrupt_contract_is_a_lifecycle_concern() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("capability-contract.json"),
    )?)?;
    let concern = contract["concerns"]
        .as_array()
        .and_then(|concerns| {
            concerns
                .iter()
                .find(|item| item["concernId"] == "watcher-wait-interruption")
        })
        .ok_or("watcher lifecycle concern")?;
    assert_eq!(concern["trigger"], MATCHER);
    assert_eq!(concern["events"], serde_json::json!(["PreToolUse", "Interrupt"]));
    assert_eq!(concern["preventive"], false);
    assert_eq!(
        concern["inputContract"],
        "codexy.hooks.watcher-wait-interruption.v1"
    );
    assert_eq!(
        concern["entrypoints"],
        serde_json::json!([
            "codexy-watcher-interrupt.sh",
            "codexy-watcher-interrupt.cmd",
            "codexy_watcher_interrupt.py"
        ])
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn watcher_hook_resolves_the_standard_cached_runtime_without_installing() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let cache = temp.path().join("runtime-cache");
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let release = manifest["version"].as_str().ok_or("plugin release")?;
    let key_input = [
        "codexy.runtime-cache/v2",
        "https://github.com/eunsoogi/codexy",
        "",
        "linux-x86_64",
        "stdio-newline-v1",
        "package-default\n",
        release,
        "codexy-mcp-watcher",
    ]
    .join("\0");
    let key = format!("v2-{:x}", Sha256::digest(key_input.as_bytes()));
    let install_root = cache.join(key);
    let runtime = install_root.join("bin/codexy-mcp-watcher");
    std::fs::create_dir_all(runtime.parent().ok_or("runtime parent")?)?;
    std::fs::copy(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
        install_root.join("plugin.json"),
    )?;
    std::fs::write(
        &runtime,
        "#!/bin/sh\nprintf '%s' '{\"requestBinding\":\"cached-binding\"}'\n",
    )?;
    let mut permissions = std::fs::metadata(&runtime)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&runtime, permissions)?;

    let output = run_pretool_hook(&root, &cache)?;
    assert!(output.status.success(), "hook failed: {:?}", output.stderr);
    let response: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        response["hookSpecificOutput"]["updatedInput"]["requestBinding"],
        "cached-binding"
    );

    std::fs::write(
        install_root.join("plugin.json"),
        r#"{"name":"codexy","repository":"https://github.com/eunsoogi/codexy","version":"0.0.0"}"#,
    )?;
    let stale_output = run_pretool_hook(&root, &cache)?;
    assert!(stale_output.status.success());
    assert!(stale_output.stdout.is_empty());

    std::fs::copy(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
        install_root.join("plugin.json"),
    )?;
    let mut permissions = std::fs::metadata(&runtime)?.permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&runtime, permissions)?;
    let non_executable = run_pretool_hook(&root, &cache)?;
    assert!(non_executable.status.success());
    assert!(non_executable.stdout.is_empty());

    std::fs::remove_file(&runtime)?;
    let missing = run_pretool_hook(&root, &cache)?;
    assert!(missing.status.success());
    assert!(missing.stdout.is_empty());
    Ok(())
}
