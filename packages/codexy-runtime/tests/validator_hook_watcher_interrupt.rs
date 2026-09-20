use serde_json::Value;

#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::{Child, Command, Stdio};

#[cfg(unix)]
#[path = "mcp_stdio/client.rs"]
mod mcp_client;
#[cfg(unix)]
use mcp_client::McpClient;
#[cfg(unix)]
#[path = "mcp_stdio/watcher_state.rs"]
mod watcher_state;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MATCHER: &str = "^(?:mcp__[^ ]+__)?watcher_wait$";

#[cfg(unix)]
fn run_hook(
    plugin_root: &Path,
    cache: &Path,
    state: Option<&Path>,
    runtime_dir: Option<&Path>,
    platform: &str,
    event: &str,
    payload: Value,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let mut command = Command::new("/bin/sh");
    command
        .arg(plugin_root.join("hooks/codexy-watcher-interrupt.sh"))
        .arg(event)
        .env_clear()
        .env("PLUGIN_ROOT", plugin_root)
        .env("CODEXY_RUNTIME_CACHE_DIR", cache)
        .env("CODEXY_RUNTIME_PLATFORM", platform)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(state) = state {
        command.env("CODEXY_WATCHER_STATE_DIR", state);
    }
    if let Some(runtime_dir) = runtime_dir {
        command.env("CODEXY_RUNTIME_DIR", runtime_dir);
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(&payload)?)?;
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
fn watcher_hook_resolves_and_interrupts_the_standard_cached_runtime() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let plugin = root.join("plugins/codexy");
    let temp = tempfile::tempdir()?;
    let cache = temp.path().join("runtime-cache");
    let state = temp.path().join("state");
    let source = Path::new(env!("CARGO_BIN_EXE_codexy-mcp-watcher"));
    install_cached(&plugin, &cache, "linux-x86_64", source)?;
    install_cached(&plugin, &cache, "invalid-platform", source)?;

    let mut setup = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut setup)?;
    let (session, parent_token, _) = watcher_state::open_session(&mut setup, "cached-hook", 2)?;
    drop(setup);
    let input = json!({
        "hook_event_name":"PreToolUse","tool_name":"mcp__codexy-watcher__watcher_wait",
        "session_id":"main-session","turn_id":"turn-1","tool_use_id":"tool-1",
        "tool_input":{"sessionId":session,"parentToken":parent_token}
    });
    let output = run_hook(&plugin, &cache, Some(&state), None, "linux-x86_64", "PreToolUse", input)?;
    assert!(output.status.success());
    let binding: Value = serde_json::from_slice(&output.stdout)?;
    let binding = binding["hookSpecificOutput"]["updatedInput"]["requestBinding"]
        .as_str().ok_or("request binding")?.to_owned();
    for (main_session, turn) in [("other-session", "turn-1"), ("main-session", "other-turn")] {
        let output = run_hook(&plugin, &cache, Some(&state), None, "linux-x86_64", "Interrupt",
            json!({"hook_event_name":"Interrupt","session_id":main_session,"turn_id":turn}))?;
        assert!(output.status.success() && output.stdout.is_empty());
        assert!(!binding_cancelled(&state, &binding));
    }
    let output = run_hook(&plugin, &cache, Some(&state), None, "linux-x86_64", "Interrupt",
        json!({"hook_event_name":"Interrupt","session_id":"main-session","turn_id":"turn-1"}))?;
    assert!(output.status.success() && output.stdout.is_empty());
    assert!(binding_cancelled(&state, &binding));
    let mut reader = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut reader)?;
    let waited = reader.send(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{"sessionId":session,"parentToken":parent_token,
        "timeoutMs":0,"requestBinding":binding}}}))?;
    assert_eq!(watcher_state::tool_payload(&waited)?["status"], "cancelled");
    let unsupported = run_hook(&plugin, &cache, None, None, "invalid-platform", "PreToolUse",
        json!({"hook_event_name":"PreToolUse","tool_name":"watcher_wait","tool_input":{}}))?;
    assert!(unsupported.status.success() && unsupported.stdout.is_empty());
    Ok(())
}

#[cfg(unix)]
fn install_cached(plugin: &Path, cache: &Path, platform: &str, source: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest = plugin.join(".codex-plugin/plugin.json");
    let release = serde_json::from_str::<Value>(&std::fs::read_to_string(&manifest)?)?["version"]
        .as_str().ok_or("plugin release")?.to_owned();
    let key = ["codexy.runtime-cache/v2", "https://github.com/eunsoogi/codexy", "", platform,
        "stdio-newline-v1", "package-default\n", &release, "codexy-mcp-watcher"].join("\0");
    let root = cache.join(format!("v2-{:x}", Sha256::digest(key.as_bytes())));
    let runtime = root.join("bin/codexy-mcp-watcher");
    std::fs::create_dir_all(runtime.parent().ok_or("runtime parent")?)?;
    std::fs::copy(&manifest, root.join("plugin.json"))?;
    std::fs::copy(source, &runtime)?;
    let mut permissions = std::fs::metadata(&runtime)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&runtime, permissions)?;
    Ok(runtime)
}

#[cfg(unix)]
fn binding_cancelled(state: &Path, nonce: &str) -> bool {
    state.join("codexy-watcher/.request-bindings").join(format!("{nonce}.cancel")).is_file()
}

#[cfg(unix)]
fn copy_plugin(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for relative in [".codex-plugin/plugin.json", "hooks/codexy-hook-runtime.sh",
        "hooks/codexy-watcher-interrupt.sh", "hooks/codexy_watcher_interrupt.py"] {
        let destination = target.join(relative);
        std::fs::create_dir_all(destination.parent().ok_or("plugin parent")?)?;
        std::fs::copy(source.join(relative), destination)?;
    }
    Ok(())
}

#[cfg(unix)]
fn fake_runtime(path: &Path, binding: &str) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(path, format!("#!/bin/sh\nprintf '%s' '{{\"requestBinding\":\"{binding}\"}}'\n"))?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn watcher_hook_preserves_override_and_bundled_runtime_precedence() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let plugin = temp.path().join("codexy");
    copy_plugin(&root.join("plugins/codexy"), &plugin)?;
    let cache = temp.path().join("cache");
    let cached = temp.path().join("cached");
    fake_runtime(&cached, "cached-binding")?;
    install_cached(&plugin, &cache, "linux-x86_64", &cached)?;
    let bundled = plugin.join("runtime/codexy-mcp-watcher-linux-x86_64.bin");
    std::fs::create_dir_all(bundled.parent().ok_or("runtime parent")?)?;
    fake_runtime(&bundled, "bundled-binding")?;
    let payload = json!({"hook_event_name":"PreToolUse","tool_name":"watcher_wait","tool_input":{}});
    let bundled_output = run_hook(&plugin, &cache, None, None, "linux-x86_64", "PreToolUse", payload.clone())?;
    let bundled_response: Value = serde_json::from_slice(&bundled_output.stdout)?;
    assert_eq!(bundled_response["hookSpecificOutput"]["updatedInput"]["requestBinding"], "bundled-binding");
    let override_dir = temp.path().join("override");
    std::fs::create_dir_all(&override_dir)?;
    fake_runtime(&override_dir.join("codexy-mcp-watcher-linux-x86_64.bin"), "override-binding")?;
    let output = run_hook(&plugin, &cache, None, Some(&override_dir), "linux-x86_64", "PreToolUse", payload)?;
    let response: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(response["hookSpecificOutput"]["updatedInput"]["requestBinding"], "override-binding");
    Ok(())
}
