use super::*;
use std::thread;
use std::time::Duration;

#[cfg(unix)]
#[test]
fn opt_in_trace_reports_hook_and_wait_boundaries_without_values() -> TestResult {
    let fixture = super::active_wait::trace_fixture()?;
    let super::active_wait::TraceFixture {
        _temp: temp,
        plugin,
        state,
        trace,
        runtime_dir,
        session,
        parent_token,
    } = fixture;
    assert_eq!(std::fs::read_dir(&trace)?.count(), 0);

    let mut disabled_client = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut disabled_client)?;
    let disabled = disabled_client.send(&json!({
        "jsonrpc":"2.0", "id":2, "method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":0
        }}
    }))?;
    let disabled = watcher_state::tool_payload(&disabled)?;
    assert_eq!(disabled["status"], "timeout");
    assert_eq!(disabled["nextCursor"], "0");
    assert_eq!(std::fs::read_dir(&trace)?.count(), 0);
    drop(disabled_client);

    let input = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"mcp__codexy-watcher__watcher_wait",
        "session_id":"private-host-session",
        "turn_id":"private-host-turn",
        "tool_use_id":"private-tool-use",
        "prompt":"private-prompt",
        "tool_input":{"sessionId":session,"parentToken":parent_token}
    });
    let hook = run_hook_with_trace(
        &plugin,
        &temp.path().join("cache"),
        &state,
        &runtime_dir,
        Some(&trace),
        "linux-x86_64",
        "PreToolUse",
        input.clone(),
    )?;
    assert!(hook.status.success());
    let hook_output: Value = serde_json::from_slice(&hook.stdout)?;
    let binding = hook_output["hookSpecificOutput"]["updatedInput"]["requestBinding"]
        .as_str()
        .ok_or("request binding")?
        .to_owned();

    let mut client = watcher_state::watcher_client_with_trace(&state, Some(&trace))?;
    watcher_state::initialize(&mut client)?;
    let bound = client.send(&json!({
        "jsonrpc":"2.0", "id":3, "method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,
            "requestBinding":binding,"timeoutMs":0
        }}
    }))?;
    let bound = watcher_state::tool_payload(&bound)?;
    assert_eq!(bound["status"], "timeout");
    assert_eq!(bound["nextCursor"], "0");
    let unbound = client.send(&json!({
        "jsonrpc":"2.0", "id":4, "method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":0
        }}
    }))?;
    let unbound = watcher_state::tool_payload(&unbound)?;
    assert_eq!(unbound["status"], "timeout");
    assert_eq!(unbound["nextCursor"], "0");

    client.send_without_read(&json!({
        "jsonrpc":"2.0", "id":5, "method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":10_000
        }}
    }))?;
    let mut observer = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut observer)?;
    let mut waiting = false;
    for request_id in 20..120 {
        let health = observer.send(&json!({
            "jsonrpc":"2.0", "id":request_id, "method":"tools/call",
            "params":{"name":"watcher_health","arguments":{
                "sessionId":session,"token":parent_token
            }}
        }))?;
        waiting = watcher_state::tool_payload(&health)?["waiting"] == true;
        if waiting {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(waiting, "trace waiter did not become active before transport cancellation");
    client.send_without_read(&json!({
        "jsonrpc":"2.0", "method":"notifications/cancelled",
        "params":{"requestId":5,"reason":"test cancellation"}
    }))?;
    let mut stopped = false;
    for request_id in 120..220 {
        let health = observer.send(&json!({
            "jsonrpc":"2.0", "id":request_id, "method":"tools/call",
            "params":{"name":"watcher_health","arguments":{
                "sessionId":session,"token":parent_token
            }}
        }))?;
        stopped = watcher_state::tool_payload(&health)?["waiting"] == false;
        if stopped {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(stopped, "transport cancellation did not release the trace waiter");
    drop(client);
    drop(observer);

    let mut trace_text = String::new();
    let mut records = Vec::new();
    let entries = std::fs::read_dir(&trace)?.collect::<Result<Vec<_>, _>>()?;
    assert!(!entries.is_empty());
    assert!(entries.len() <= 32);
    assert_eq!(std::fs::metadata(&trace)?.permissions().mode() & 0o077, 0);
    for entry in entries {
        let path = entry.path();
        assert!(path.file_name().and_then(|name| name.to_str()).is_some_and(|name|
            name.starts_with("slot-") && name.ends_with(".jsonl")));
        let metadata = std::fs::symlink_metadata(&path)?;
        assert!(metadata.file_type().is_file());
        assert_eq!(metadata.permissions().mode() & 0o077, 0);
        assert!(metadata.len() <= 4096);
        let content = std::fs::read_to_string(path)?;
        trace_text.push_str(&content);
        assert!(content.lines().count() <= 4);
        for line in content.lines() {
            assert!(line.len() <= 512);
            records.push(serde_json::from_str::<Value>(line)?);
        }
    }
    assert!(records.iter().any(|record| record["event"] == "hook_received"
        && record["eventKind"] == "PreToolUse"
        && record["toolNameMatched"] == true
        && record["toolInputPresent"] == true));
    assert!(records.iter().any(|record| record["event"] == "hook_received"
        && record["parentTokenPresent"] == true));
    assert!(records.iter().any(|record| record["event"] == "binding_output"
        && record["bindingPresent"] == true));
    assert!(records.iter().any(|record| record["event"] == "wait_received"
        && record["bindingPresent"] == true));
    assert!(records.iter().any(|record| record["event"] == "binding_claim"
        && record["claimAttempted"] == true
        && record["claimSucceeded"] == true));
    assert!(records.iter().any(|record| record["event"] == "wait_received"
        && record["bindingPresent"] == false));
    assert!(records.iter().any(|record| record["event"] == "binding_claim"
        && record["claimAttempted"] == false
        && record["claimSucceeded"] == false));
    assert_eq!(records.iter().filter(|record|
        record["event"] == "wait_ended" && record["endCause"] == "timeout").count(), 2);
    assert_eq!(records.iter().filter(|record|
        record["event"] == "wait_ended" && record["endCause"] == "transport_cancelled").count(), 1);
    assert!(!records.iter().any(|record|
        record["event"] == "wait_failed" && record["failureClass"] == "wait_error"));
    for secret in [
        session.as_str(),
        parent_token.as_str(),
        binding.as_str(),
        "private-host-session",
        "private-host-turn",
        "private-tool-use",
        "private-prompt",
    ] {
        assert!(!trace_text.contains(secret));
    }

    let unsafe_trace = temp.path().join("unsafe-trace");
    std::fs::create_dir(&unsafe_trace)?;
    std::fs::set_permissions(&unsafe_trace, PermissionsExt::from_mode(0o755))?;
    let mut failure_input = input.clone();
    failure_input["turn_id"] = json!("another-private-turn");
    failure_input["tool_use_id"] = json!("another-private-tool-use");
    let hook_without_logging = run_hook_with_trace(
        &plugin,
        &temp.path().join("cache"),
        &state,
        &runtime_dir,
        Some(&unsafe_trace),
        "linux-x86_64",
        "PreToolUse",
        failure_input,
    )?;
    assert!(hook_without_logging.status.success());
    assert!(!hook_without_logging.stdout.is_empty());
    assert_eq!(std::fs::read_dir(&unsafe_trace)?.count(), 0);

    let mut unsafe_client = watcher_state::watcher_client_with_trace(&state, Some(&unsafe_trace))?;
    watcher_state::initialize(&mut unsafe_client)?;
    let unaffected = unsafe_client.send(&json!({
        "jsonrpc":"2.0", "id":5, "method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":0
        }}
    }))?;
    assert_eq!(watcher_state::tool_payload(&unaffected)?["status"], "timeout");
    assert_eq!(std::fs::read_dir(&unsafe_trace)?.count(), 0);

    let symlink_target = temp.path().join("symlink-target");
    std::fs::create_dir(&symlink_target)?;
    std::fs::set_permissions(&symlink_target, PermissionsExt::from_mode(0o700))?;
    let symlink_trace = temp.path().join("symlink-trace");
    std::os::unix::fs::symlink(&symlink_target, &symlink_trace)?;
    let mut symlink_input = input;
    symlink_input["turn_id"] = json!("symlink-private-turn");
    symlink_input["tool_use_id"] = json!("symlink-private-tool-use");
    let hook_without_symlink_logging = run_hook_with_trace(
        &plugin,
        &temp.path().join("cache"),
        &state,
        &runtime_dir,
        Some(&symlink_trace),
        "linux-x86_64",
        "PreToolUse",
        symlink_input,
    )?;
    assert!(hook_without_symlink_logging.status.success());
    assert!(!hook_without_symlink_logging.stdout.is_empty());
    assert_eq!(std::fs::read_dir(&symlink_target)?.count(), 0);

    let mut symlink_client = watcher_state::watcher_client_with_trace(&state, Some(&symlink_trace))?;
    watcher_state::initialize(&mut symlink_client)?;
    let unaffected = symlink_client.send(&json!({
        "jsonrpc":"2.0", "id":6, "method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":0
        }}
    }))?;
    assert_eq!(watcher_state::tool_payload(&unaffected)?["status"], "timeout");
    assert_eq!(std::fs::read_dir(&symlink_target)?.count(), 0);
    Ok(())
}
