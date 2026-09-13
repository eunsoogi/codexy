use super::*;
use super::watcher_state::{initialize, open_session, tool_payload, watcher_client};
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LONG_WAIT_MS: u64 = 3_600_000;

fn waiting_until_true(
    observer: &mut McpClient,
    session: &str,
    parent_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for request_id in 10..110 {
        let health = observer.send(&json!({
            "jsonrpc": "2.0", "id": request_id, "method": "tools/call",
            "params": {"name": "watcher_health", "arguments": {
                "sessionId": session, "token": parent_token
            }}
        }))?;
        if tool_payload(&health)?["waiting"] == true {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(1));
    }
    Err("waiter did not acquire wait.lock".into())
}

#[test]
fn wait_schema_exposes_an_optional_host_interrupt_binding() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let listed = client.send(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
    }))?;
    let wait = listed["result"]["tools"]
        .as_array()
        .and_then(|tools| tools.iter().find(|tool| tool["name"] == "watcher_wait"))
        .ok_or("watcher_wait schema is missing")?;
    let binding = &wait["inputSchema"]["properties"]["requestBinding"];
    assert_eq!(binding["type"], "string");
    assert_eq!(binding["maxLength"], 128);
    Ok(())
}

#[test]
fn native_interrupt_releases_only_the_bound_wait_and_preserves_the_session() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, _) = open_session(&mut setup, "native-interrupt", 2)?;
    drop(setup);

    let binding = hook_call(
        state.path(),
        "--hook-pretool",
        json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__codexy-watcher__watcher_wait",
            "session_id": "main-session",
            "turn_id": "turn-1",
            "tool_use_id": "tool-1",
            "tool_input": {"sessionId": session, "parentToken": parent_token, "timeoutMs": LONG_WAIT_MS}
        }),
    )?["requestBinding"]
        .as_str()
        .ok_or("missing request binding")?
        .to_owned();

    let mut reader = watcher_client(state.path())?;
    initialize(&mut reader)?;
    reader.send_without_read(&json!({
        "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": {"name": "watcher_wait", "arguments": {
            "sessionId": session, "parentToken": parent_token,
            "timeoutMs": LONG_WAIT_MS, "requestBinding": binding
        }}
    }))?;
    let mut observer = watcher_client(state.path())?;
    initialize(&mut observer)?;
    waiting_until_true(&mut observer, &session, &parent_token)?;
    let record = binding_record(state.path(), &binding)?;
    assert_eq!(record["status"], "active");
    assert!(record["expiresAtMs"].as_u64().unwrap() >= now_ms() + LONG_WAIT_MS - 1_000);

    let stale = hook_call(
        state.path(), "--hook-interrupt",
        json!({"hook_event_name": "Interrupt", "session_id": "wrong", "turn_id": "turn-1"}),
    )?;
    assert_eq!(stale["cancelled"], false);
    let wrong_turn = hook_call(
        state.path(), "--hook-interrupt",
        json!({"hook_event_name": "Interrupt", "session_id": "main-session", "turn_id": "wrong-turn"}),
    )?;
    assert_eq!(wrong_turn["cancelled"], false);
    let started = Instant::now();
    let interrupt = hook_call(
        state.path(), "--hook-interrupt",
        json!({"hook_event_name": "Interrupt", "session_id": "main-session", "turn_id": "turn-1"}),
    )?;
    assert_eq!(interrupt["cancelled"], true);
    let waited = tool_payload(&reader.read_frame()?)?;
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(waited["status"], "cancelled");
    assert_eq!(waited["nextCursor"], "0");

    let mut replacement = watcher_client(state.path())?;
    initialize(&mut replacement)?;
    let health = replacement.send(&json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": {"name": "watcher_health", "arguments": {
            "sessionId": session, "token": parent_token
        }}
    }))?;
    assert_eq!(tool_payload(&health)?["status"], "active");
    assert_eq!(tool_payload(&health)?["waiting"], false);
    let after_completion = hook_call(
        state.path(), "--hook-interrupt",
        json!({"hook_event_name": "Interrupt", "session_id": "main-session", "turn_id": "turn-1"}),
    )?;
    assert_eq!(after_completion["cancelled"], false);
    Ok(())
}

#[test]
fn armed_interrupt_is_preserved_until_wait_claims() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, _) = open_session(&mut setup, "armed-interrupt", 2)?;
    drop(setup);

    let binding = hook_call(
        state.path(),
        "--hook-pretool",
        json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__codexy-watcher__watcher_wait",
            "session_id": "armed-main",
            "turn_id": "armed-turn",
            "tool_use_id": "armed-tool",
            "tool_input": {"sessionId": session, "parentToken": parent_token}
        }),
    )?["requestBinding"]
        .as_str()
        .ok_or("missing request binding")?
        .to_owned();

    let wrong_turn = hook_call(
        state.path(),
        "--hook-interrupt",
        json!({"hook_event_name": "Interrupt", "session_id": "armed-main", "turn_id": "other-turn"}),
    )?;
    assert_eq!(wrong_turn["cancelled"], false);
    let interrupt = hook_call(
        state.path(),
        "--hook-interrupt",
        json!({"hook_event_name": "Interrupt", "session_id": "armed-main", "turn_id": "armed-turn"}),
    )?;
    assert_eq!(interrupt["cancelled"], true);

    let mut reader = watcher_client(state.path())?;
    initialize(&mut reader)?;
    let waited = reader.send(&json!({
        "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": {"name": "watcher_wait", "arguments": {
            "sessionId": session, "parentToken": parent_token,
            "timeoutMs": LONG_WAIT_MS, "requestBinding": binding
        }}
    }))?;
    assert_eq!(tool_payload(&waited)?["status"], "cancelled");
    assert_eq!(tool_payload(&waited)?["nextCursor"], "0");
    Ok(())
}

fn binding_record(state: &std::path::Path, nonce: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = state
        .join("codexy-watcher/.request-bindings")
        .join(format!("{nonce}.json"));
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_millis() as u64
}

fn hook_call(
    state: &std::path::Path,
    action: &str,
    payload: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-watcher"))
        .arg(action)
        .env("CODEXY_WATCHER_STATE_DIR", state)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(&payload)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "hook failed: {:?}", output.stderr);
    Ok(serde_json::from_slice(&output.stdout)?)
}
