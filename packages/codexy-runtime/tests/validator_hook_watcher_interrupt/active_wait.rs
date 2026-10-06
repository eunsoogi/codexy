use super::*;
use std::thread;
use std::time::Duration;

// Only the matching host session and turn release this bound wait; watcher
// state remains usable afterward, and unsupported runtime resolution stays quiet.
#[cfg(unix)]
#[test]
fn watcher_hook_resolves_and_interrupts_the_standard_cached_runtime() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let plugin = root.join("plugins/codexy");
    let temp = tempfile::tempdir()?;
    let cache = temp.path().join("runtime-cache");
    let state = temp.path().join("state");
    let source = Path::new(env!("CARGO_BIN_EXE_codexy-mcp-watcher"));
    let runtime = install_cached(&plugin, &cache, "linux-x86_64", source)?;
    let probe = temp.path().join("invalid-probe");
    fake_runtime(&probe, "invalid-platform-binding")?;
    install_cached(&plugin, &cache, "invalid-platform", &probe)?;

    let mut setup = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut setup)?;
    let (session, parent_token, watcher_token) =
        watcher_state::open_session(&mut setup, "cached-hook", 2)?;
    drop(setup);
    let input = json!({
        "hook_event_name":"PreToolUse","tool_name":"mcp__codexy-watcher__watcher_wait",
        "session_id":"main-session","turn_id":"turn-1","tool_use_id":"tool-1",
        "tool_input":{"sessionId":session,"parentToken":parent_token}
    });
    let output = run_hook(&plugin, &cache, &state, Path::new(""), "linux-x86_64", "PreToolUse", input.clone())?;
    assert!(output.status.success());
    let binding: Value = serde_json::from_slice(&output.stdout)?;
    let binding = binding["hookSpecificOutput"]["updatedInput"]["requestBinding"]
        .as_str().ok_or("request binding")?.to_owned();
    let mut reader = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut reader)?;
    reader.send_without_read(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{"sessionId":session,
        "parentToken":parent_token,"requestBinding":binding}}}))?;
    let mut observer = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut observer)?;
    let mut waiting = false;
    for request_id in 10..110 {
        let health = observer.send(&json!({"jsonrpc":"2.0","id":request_id,"method":"tools/call",
            "params":{"name":"watcher_health","arguments":{"sessionId":session,"token":parent_token}}}))?;
        waiting = watcher_state::tool_payload(&health)?["waiting"].as_bool().unwrap_or(false);
        if waiting {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(waiting, "bound watcher_wait did not become active");
    for (main_session, turn) in [("other-session", "turn-1"), ("main-session", "other-turn")] {
        let output = run_hook(&plugin, &cache, &state, Path::new(""), "linux-x86_64", "Interrupt",
            json!({"hook_event_name":"Interrupt","session_id":main_session,"turn_id":turn}))?;
        assert!(output.status.success() && output.stdout.is_empty());
        assert!(!binding_cancelled(&state, &binding));
    }
    let started = std::time::Instant::now();
    let output = run_hook(&plugin, &cache, &state, Path::new(""), "linux-x86_64", "Interrupt",
        json!({"hook_event_name":"Interrupt","session_id":"main-session","turn_id":"turn-1"}))?;
    assert!(output.status.success() && output.stdout.is_empty());
    let waited = watcher_state::tool_payload(&reader.read_frame()?)?;
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(waited["status"], "cancelled");
    assert_eq!(waited["nextCursor"], "0");
    let health = observer.send(&json!({"jsonrpc":"2.0","id":111,"method":"tools/call",
        "params":{"name":"watcher_health","arguments":{"sessionId":session,"token":parent_token}}}))?;
    let health = watcher_state::tool_payload(&health)?;
    assert_eq!(health["status"], "active");
    assert_eq!(health["waiting"], false);
    let reported = observer.send(&json!({"jsonrpc":"2.0","id":112,"method":"tools/call",
        "params":{"name":"watcher_report","arguments":{"sessionId":session,
        "watcherToken":watcher_token,"target":{"threadId":"target"},
        "eventId":"after-interrupt","kind":"gate_ready","summary":"event after interrupted wait"}}}))?;
    assert_eq!(watcher_state::tool_payload(&reported)?["status"], "accepted");
    let resumed = reader.send(&json!({"jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{"sessionId":session,
        "parentToken":parent_token,"cursor":0,"timeoutMs":0}}}))?;
    let resumed = watcher_state::tool_payload(&resumed)?;
    assert_eq!(resumed["status"], "event");
    assert_eq!(resumed["events"].as_array().ok_or("events")?.len(), 1);
    let unsupported = run_hook(&plugin, &cache, &state, Path::new(""), "invalid-platform", "PreToolUse",
        json!({"hook_event_name":"PreToolUse","tool_name":"watcher_wait","tool_input":{}}))?;
    assert!(unsupported.status.success() && unsupported.stdout.is_empty());
    let cached_manifest = runtime.parent().and_then(Path::parent).ok_or("cache root")?.join("plugin.json");
    std::fs::write(&cached_manifest, r#"{"name":"codexy","repository":"https://github.com/eunsoogi/codexy","version":"0.0.0"}"#)?;
    let output = run_hook(&plugin, &cache, &state, Path::new(""), "linux-x86_64", "PreToolUse", input.clone())?;
    assert!(output.status.success() && output.stdout.is_empty());
    std::fs::copy(plugin.join(".codex-plugin/plugin.json"), &cached_manifest)?;
    std::fs::set_permissions(&runtime, PermissionsExt::from_mode(0o644))?;
    let output = run_hook(&plugin, &cache, &state, Path::new(""), "linux-x86_64", "PreToolUse", input.clone())?;
    assert!(output.status.success() && output.stdout.is_empty());
    std::fs::set_permissions(&runtime, PermissionsExt::from_mode(0o755))?;
    std::fs::remove_file(&runtime)?;
    let output = run_hook(&plugin, &cache, &state, Path::new(""), "linux-x86_64", "PreToolUse", input)?;
    assert!(output.status.success() && output.stdout.is_empty());
    Ok(())
}



pub(super) struct TraceFixture {
    pub(super) _temp: tempfile::TempDir,
    pub(super) plugin: PathBuf,
    pub(super) state: PathBuf,
    pub(super) trace: PathBuf,
    pub(super) runtime_dir: PathBuf,
    pub(super) session: String,
    pub(super) parent_token: String,
}

pub(super) fn trace_fixture() -> Result<TraceFixture, Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let plugin = temp.path().join("plugin");
    copy_plugin(&root.join("plugins/codexy"), &plugin)?;
    let state = temp.path().join("state");
    let trace = temp.path().join("trace");
    std::fs::create_dir(&trace)?;
    std::fs::set_permissions(&trace, PermissionsExt::from_mode(0o700))?;
    let runtime_dir = temp.path().join("runtime");
    std::fs::create_dir(&runtime_dir)?;
    let runtime = runtime_dir.join("codexy-mcp-watcher-linux-x86_64.bin");
    std::fs::copy(
        Path::new(env!("CARGO_BIN_EXE_codexy-mcp-watcher")),
        &runtime,
    )?;
    std::fs::set_permissions(&runtime, PermissionsExt::from_mode(0o755))?;

    let mut setup = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut setup)?;
    let (session, parent_token, _) = watcher_state::open_session(&mut setup, "trace-boundary", 2)?;
    drop(setup);
    Ok(TraceFixture {
        _temp: temp,
        plugin,
        state,
        trace,
        runtime_dir,
        session,
        parent_token,
    })
}
