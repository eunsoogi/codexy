use super::{
    LONG_WAIT_MS, TestResult, active_binding_until_true, binding_gone_until_true, binding_record,
    hook_call, now_ms, waiting_until_true, write_binding_record,
};
use super::super::watcher_state::{initialize, open_session, tool_payload, watcher_client};
use serde_json::json;
use std::time::{Duration, Instant};

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
    let record = active_binding_until_true(state.path(), &binding)?;
    assert_eq!(record["status"], "active");
    assert!(record["expiresAtMs"].as_u64().unwrap() >= now_ms() + LONG_WAIT_MS - 1_000);

    // The wall-clock expiry is deliberately behind us while the real waiter
    // still owns wait.lock. Interrupt must retain the active binding until
    // that waiter observes cancellation and exits.
    let mut expired_active = record.clone();
    let now = now_ms();
    expired_active["createdAtMs"] = json!(now.saturating_sub(LONG_WAIT_MS));
    expired_active["expiresAtMs"] = json!(now.saturating_sub(1));
    write_binding_record(state.path(), &binding, &expired_active)?;

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
    assert!(binding_gone_until_true(state.path(), &binding));

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
    let replay = replacement.send(&json!({
        "jsonrpc": "2.0", "id": 6, "method": "tools/call",
        "params": {"name": "watcher_wait", "arguments": {
            "sessionId": session, "parentToken": parent_token,
            "timeoutMs": 0, "requestBinding": binding
        }}
    }))?;
    assert!(
        replay.get("error").is_some(),
        "completed binding was replayable: {replay}"
    );
    let after_completion = hook_call(
        state.path(), "--hook-interrupt",
        json!({"hook_event_name": "Interrupt", "session_id": "main-session", "turn_id": "turn-1"}),
    )?;
    assert_eq!(after_completion["cancelled"], false);
    Ok(())
}

#[test]
fn completed_bound_wait_reclaims_its_active_binding() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, _) = open_session(&mut setup, "completed-binding", 2)?;
    drop(setup);

    let binding = hook_call(
        state.path(),
        "--hook-pretool",
        json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__codexy-watcher__watcher_wait",
            "session_id": "completed-main",
            "turn_id": "completed-turn",
            "tool_use_id": "completed-tool",
            "tool_input": {"sessionId": session, "parentToken": parent_token}
        }),
    )?["requestBinding"]
        .as_str()
        .ok_or("missing request binding")?
        .to_owned();

    let mut reader = watcher_client(state.path())?;
    initialize(&mut reader)?;
    let completed = reader.send(&json!({
        "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": {"name": "watcher_wait", "arguments": {
            "sessionId": session, "parentToken": parent_token,
            "timeoutMs": 0, "requestBinding": binding
        }}
    }))?;
    assert_eq!(tool_payload(&completed)?["status"], "timeout");
    assert!(binding_gone_until_true(state.path(), &binding));
    Ok(())
}

#[test]
fn expired_armed_binding_is_rejected_and_reclaimed() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, _) = open_session(&mut setup, "expired-armed", 2)?;
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
    let mut expired = binding_record(state.path(), &binding)?;
    let now = now_ms();
    expired["createdAtMs"] = json!(now.saturating_sub(1_000));
    expired["expiresAtMs"] = json!(now.saturating_sub(1));
    write_binding_record(state.path(), &binding, &expired)?;

    let mut reader = watcher_client(state.path())?;
    initialize(&mut reader)?;
    let rejected = reader.send(&json!({
        "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": {"name": "watcher_wait", "arguments": {
            "sessionId": session, "parentToken": parent_token,
            "timeoutMs": 0, "requestBinding": binding
        }}
    }))?;
    assert!(
        rejected.get("error").is_some(),
        "expired binding was accepted: {rejected}"
    );
    assert_eq!(binding_record(state.path(), &binding)?["status"], "armed");

    let interrupt = hook_call(
        state.path(),
        "--hook-interrupt",
        json!({"hook_event_name": "Interrupt", "session_id": "armed-main", "turn_id": "armed-turn"}),
    )?;
    assert_eq!(interrupt["cancelled"], false);
    assert!(binding_gone_until_true(state.path(), &binding));
    Ok(())
}
