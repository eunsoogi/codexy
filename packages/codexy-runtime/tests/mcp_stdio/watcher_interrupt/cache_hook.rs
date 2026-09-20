use super::super::fixtures;
use super::super::watcher_state::{initialize, open_session, tool_payload, watcher_client};
use super::TestResult;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn install_standard_cached_watcher(
    plugin_root: &Path,
    cache: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = plugin_root.join(".codex-plugin/plugin.json");
    let release = serde_json::from_str::<Value>(&fs::read_to_string(&manifest)?)?["version"]
        .as_str()
        .ok_or("plugin release")?
        .to_owned();
    let key_input = [
        "codexy.runtime-cache/v2",
        "https://github.com/eunsoogi/codexy",
        "",
        "linux-x86_64",
        "stdio-newline-v1",
        "package-default\n",
        &release,
        "codexy-mcp-watcher",
    ]
    .join("\0");
    let install_root = cache.join(format!("v2-{:x}", Sha256::digest(key_input.as_bytes())));
    let runtime = install_root.join("bin/codexy-mcp-watcher");
    fs::create_dir_all(runtime.parent().ok_or("runtime parent")?)?;
    fs::copy(&manifest, install_root.join("plugin.json"))?;
    fs::copy(env!("CARGO_BIN_EXE_codexy-mcp-watcher"), &runtime)?;
    use std::os::unix::fs::PermissionsExt as _;
    let mut permissions = fs::metadata(&runtime)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(runtime, permissions)?;
    Ok(())
}

fn shell_hook_call(
    plugin_root: &Path,
    cache: &Path,
    state: &Path,
    runtime_dir: Option<&Path>,
    event: &str,
    payload: Value,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let mut command = Command::new("/bin/sh");
    command
        .arg(plugin_root.join("hooks/codexy-watcher-interrupt.sh"))
        .arg(event)
        .env_clear()
        .env("PLUGIN_ROOT", plugin_root)
        .env("CODEXY_RUNTIME_CACHE_DIR", cache)
        .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
        .env("CODEXY_WATCHER_STATE_DIR", state)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(runtime_dir) = runtime_dir {
        command.env("CODEXY_RUNTIME_DIR", runtime_dir);
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(&payload)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "hook failed: {:?}", output.stderr);
    if output.stdout.is_empty() {
        return Ok(None);
    }
    serde_json::from_slice(&output.stdout)
        .map(Some)
        .map_err(|error| {
            format!(
                "{event} hook returned invalid JSON: {error}; stdout={:?}; stderr={:?}",
                output.stdout, output.stderr
            )
            .into()
        })
}

fn binding_cancelled(state: &Path, nonce: &str) -> bool {
    state
        .join("codexy-watcher/.request-bindings")
        .join(format!("{nonce}.cancel"))
        .is_file()
}

fn write_fake_runtime(path: &Path, binding: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        path,
        format!("#!/bin/sh\nprintf '%s' '{{\"requestBinding\":\"{binding}\"}}'\n"),
    )?;
    use std::os::unix::fs::PermissionsExt as _;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[test]
fn standard_cached_runtime_hook_binds_and_interrupts_the_matching_turn() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let plugin_root = root.join("plugins/codexy");
    let cache = tempfile::tempdir()?;
    let state = tempfile::tempdir()?;
    install_standard_cached_watcher(&plugin_root, cache.path())?;

    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, _) = open_session(&mut setup, "cached-hook", 2)?;
    drop(setup);

    let pretool = shell_hook_call(
        &plugin_root,
        cache.path(),
        state.path(),
        None,
        "PreToolUse",
        json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__codexy-watcher__watcher_wait",
            "session_id": "main-session",
            "turn_id": "turn-1",
            "tool_use_id": "tool-1",
            "tool_input": {"sessionId": session, "parentToken": parent_token}
        }),
    )?
    .ok_or("cached PreToolUse hook returned no output")?;
    let binding = pretool["hookSpecificOutput"]["updatedInput"]["requestBinding"]
        .as_str()
        .ok_or("missing cached runtime request binding")?
        .to_owned();

    let wrong_session = shell_hook_call(
        &plugin_root,
        cache.path(),
        state.path(),
        None,
        "Interrupt",
        json!({"hook_event_name":"Interrupt","session_id":"other-session","turn_id":"turn-1"}),
    )?;
    assert!(wrong_session.is_none());
    assert!(!binding_cancelled(state.path(), &binding));
    let wrong_turn = shell_hook_call(
        &plugin_root,
        cache.path(),
        state.path(),
        None,
        "Interrupt",
        json!({"hook_event_name":"Interrupt","session_id":"main-session","turn_id":"other-turn"}),
    )?;
    assert!(wrong_turn.is_none());
    assert!(!binding_cancelled(state.path(), &binding));
    let interrupt = shell_hook_call(
        &plugin_root,
        cache.path(),
        state.path(),
        None,
        "Interrupt",
        json!({"hook_event_name":"Interrupt","session_id":"main-session","turn_id":"turn-1"}),
    )?;
    assert!(interrupt.is_none());
    assert!(binding_cancelled(state.path(), &binding));

    let mut reader = watcher_client(state.path())?;
    initialize(&mut reader)?;
    let waited = reader.send(&json!({
        "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": {"name": "watcher_wait", "arguments": {
            "sessionId": session, "parentToken": parent_token,
            "timeoutMs": 0, "requestBinding": binding
        }}
    }))?;
    assert_eq!(tool_payload(&waited)?["status"], "cancelled");
    Ok(())
}

#[test]
fn shell_hook_keeps_runtime_override_and_bundle_before_cache() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let source_plugin = root.join("plugins/codexy");
    let plugin_temp = tempfile::tempdir()?;
    let plugin_root = plugin_temp.path().join("codexy");
    fixtures::copy_dir(&source_plugin, &plugin_root)?;
    let cache = tempfile::tempdir()?;
    let state = tempfile::tempdir()?;
    install_standard_cached_watcher(&source_plugin, cache.path())?;

    let bundled = plugin_root.join("runtime/codexy-mcp-watcher-linux-x86_64.bin");
    fs::create_dir_all(bundled.parent().ok_or("bundled runtime parent")?)?;
    write_fake_runtime(&bundled, "bundled-binding")?;
    let bundled_response = shell_hook_call(
        &plugin_root,
        cache.path(),
        state.path(),
        None,
        "PreToolUse",
        json!({"hook_event_name":"PreToolUse","tool_name":"watcher_wait","tool_input":{}}),
    )?
    .ok_or("bundled PreToolUse hook returned no output")?;
    assert_eq!(
        bundled_response["hookSpecificOutput"]["updatedInput"]["requestBinding"],
        "bundled-binding"
    );

    let override_temp = tempfile::tempdir()?;
    let override_runtime = override_temp.path().join("codexy-mcp-watcher-linux-x86_64.bin");
    write_fake_runtime(&override_runtime, "override-binding")?;
    let override_response = shell_hook_call(
        &plugin_root,
        cache.path(),
        state.path(),
        Some(override_temp.path()),
        "PreToolUse",
        json!({"hook_event_name":"PreToolUse","tool_name":"watcher_wait","tool_input":{}}),
    )?
    .ok_or("override PreToolUse hook returned no output")?;
    assert_eq!(
        override_response["hookSpecificOutput"]["updatedInput"]["requestBinding"],
        "override-binding"
    );
    Ok(())
}
