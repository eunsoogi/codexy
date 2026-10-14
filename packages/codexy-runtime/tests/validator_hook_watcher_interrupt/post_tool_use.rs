use super::*;
use std::thread;
use std::time::Duration;

const TEST_WAIT_MS: u64 = 30_000;

pub(super) fn arm_binding(
    fixture: &active_wait::TraceFixture,
    main_session: &str,
    turn: &str,
    tool_use: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = run_hook(
        &fixture.plugin,
        &fixture.runtime_dir,
        &fixture.state,
        &fixture.runtime_dir,
        "linux-x86_64",
        "PreToolUse",
        json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "mcp__codexy-watcher__watcher_wait",
            "session_id": main_session,
            "turn_id": turn,
            "tool_use_id": tool_use,
            "tool_input": {
                "sessionId": fixture.session,
                "parentToken": fixture.parent_token
            }
        }),
    )?;
    assert!(output.status.success(), "wait binding setup failed: {:?}", output.stderr);
    let response: Value = serde_json::from_slice(&output.stdout)?;
    Ok(response["hookSpecificOutput"]["updatedInput"]["requestBinding"]
        .as_str()
        .ok_or("request binding")?
        .to_owned())
}

pub(super) fn posttool_payload(input_recipient: &str, output_recipient: &str) -> Value {
    json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "mcp__codex_app__send_message_to_thread",
        "session_id": "sender-thread",
        "turn_id": "sender-turn",
        "tool_use_id": "send-tool",
        "tool_input": {"threadId": input_recipient},
        "tool_response": {
            "content": [{
                "type": "text",
                "text": format!("{{\"threadId\":\"{output_recipient}\"}}")
            }],
            "isError": false
        }
    })
}

pub(super) fn assert_no_cancellation(
    fixture: &active_wait::TraceFixture,
    payload: Value,
    bindings: &[&str],
) -> TestResult {
    let output = run_hook(
        &fixture.plugin,
        &fixture.runtime_dir,
        &fixture.state,
        &fixture.runtime_dir,
        "linux-x86_64",
        "PostToolUse",
        payload,
    )?;
    assert!(output.status.success(), "PostToolUse hook failed: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "PostToolUse must not alter the sent message");
    for binding in bindings {
        assert!(!binding_cancelled(&fixture.state, binding));
    }
    Ok(())
}

fn hook_trace(fixture: &active_wait::TraceFixture) -> String {
    std::fs::read_dir(&fixture.trace)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect()
}

// A successful sender hook must release the recipient's bound wait and leave its session reusable.
#[test]
fn successful_post_tool_send_releases_recipient_wait() -> TestResult {
    let fixture = active_wait::trace_fixture()?;
    let pretool = json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "mcp__codexy-watcher__watcher_wait",
        "session_id": "recipient-thread",
        "turn_id": "recipient-turn",
        "tool_use_id": "wait-tool",
        "tool_input": {
            "sessionId": fixture.session,
            "parentToken": fixture.parent_token,
            "timeoutMs": TEST_WAIT_MS
        }
    });
    let output = run_hook(
        &fixture.plugin,
        &fixture.runtime_dir,
        &fixture.state,
        &fixture.runtime_dir,
        "linux-x86_64",
        "PreToolUse",
        pretool,
    )?;
    let binding: Value = serde_json::from_slice(&output.stdout)?;
    let binding = binding["hookSpecificOutput"]["updatedInput"]["requestBinding"]
        .as_str()
        .ok_or("request binding")?
        .to_owned();

    let mut reader = watcher_state::watcher_client(&fixture.state)?;
    watcher_state::initialize(&mut reader)?;
    reader.send_without_read(&json!({
        "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": {"name": "watcher_wait", "arguments": {
            "sessionId": fixture.session,
            "parentToken": fixture.parent_token,
            "timeoutMs": TEST_WAIT_MS,
            "requestBinding": binding
        }}
    }))?;
    let mut observer = watcher_state::watcher_client(&fixture.state)?;
    watcher_state::initialize(&mut observer)?;
    let mut waiting = false;
    for request_id in 10..110 {
        let health = observer.send(&json!({
            "jsonrpc": "2.0", "id": request_id, "method": "tools/call",
            "params": {"name": "watcher_health", "arguments": {
                "sessionId": fixture.session, "token": fixture.parent_token
            }}
        }))?;
        waiting = watcher_state::tool_payload(&health)?["waiting"] == true;
        if waiting {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(waiting, "recipient watcher_wait did not become active");

    // session_id belongs to the sender; only the matching input and response recipient may cancel.
    let posttool = json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "mcp__codex_app__send_message_to_thread",
        "session_id": "sender-thread",
        "turn_id": "sender-turn",
        "tool_use_id": "send-tool",
        "tool_input": {"threadId": "recipient-thread"},
        "tool_response": {
            "content": [{"type": "text", "text": "{\"threadId\":\"recipient-thread\"}"}],
            "isError": false
        }
    });
    let sent = run_hook_with_trace(
        &fixture.plugin,
        &fixture.runtime_dir,
        &fixture.state,
        &fixture.runtime_dir,
        Some(&fixture.trace),
        "linux-x86_64",
        "PostToolUse",
        posttool,
    )?;
    assert!(sent.status.success(), "PostToolUse hook failed: {:?}", sent.stderr);
    assert!(sent.stdout.is_empty(), "PostToolUse must not alter the sent message");
    let sender_matched = hook_trace(&fixture).contains("\"cancellationMatched\":true");
    let mut waiting_after_send = true;
    for request_id in 120..220 {
        let health = observer.send(&json!({
            "jsonrpc": "2.0", "id": request_id, "method": "tools/call",
            "params": {"name": "watcher_health", "arguments": {
                "sessionId": fixture.session,
                "token": fixture.parent_token
            }}
        }))?;
        waiting_after_send = watcher_state::tool_payload(&health)?["waiting"] == true;
        if !waiting_after_send {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    if waiting_after_send {
        // Release the waiter on regression paths so the bounded test cannot leak it.
        let _ = run_hook(
            &fixture.plugin,
            &fixture.runtime_dir,
            &fixture.state,
            &fixture.runtime_dir,
            "linux-x86_64",
            "Interrupt",
            json!({"hook_event_name":"Interrupt","session_id":"recipient-thread","turn_id":"recipient-turn"}),
        )?;
    }
    let waited = watcher_state::tool_payload(&reader.read_frame()?)?;
    assert!(
        sender_matched,
        "successful sender PostToolUse did not match the recipient binding: {}",
        hook_trace(&fixture),
    );
    assert_eq!(waited["status"], "cancelled");
    assert_eq!(waited["cancellationReason"], "request_cancelled");
    assert_eq!(waited["nextCursor"], "0");
    assert_eq!(waited["health"]["status"], "active");

    let reported = observer.send(&json!({
        "jsonrpc": "2.0", "id": 6, "method": "tools/call",
        "params": {"name": "watcher_report", "arguments": {
            "sessionId": fixture.session,
            "watcherToken": fixture.watcher_token,
            "target": {"threadId": "target"},
            "eventId": "after-message-cancellation",
            "kind": "gate_ready",
            "summary": "report after sender cancellation"
        }}
    }))?;
    assert_eq!(watcher_state::tool_payload(&reported)?["status"], "accepted");
    let reused = reader.send(&json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": {"name": "watcher_wait", "arguments": {
            "sessionId": fixture.session,
            "parentToken": fixture.parent_token,
            "cursor": "0",
            "timeoutMs": 0
        }}
    }))?;
    let resumed = watcher_state::tool_payload(&reused)?;
    assert_eq!(resumed["status"], "event");
    assert_eq!(resumed["nextCursor"], "1");
    assert_eq!(resumed["events"].as_array().ok_or("events")?.len(), 1);
    Ok(())
}
