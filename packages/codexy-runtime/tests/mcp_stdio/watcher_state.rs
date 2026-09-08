use super::*;
use std::sync::{Arc, Barrier};
use std::thread;

pub(super) fn watcher_client(state_dir: &Path) -> Result<McpClient, Box<dyn std::error::Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-watcher"));
    command
        .env("CODEXY_WATCHER_STATE_DIR", state_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    McpClient::spawn_command(command)
}

pub(super) fn tool_payload(response: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let text = response
        .get("result")
        .and_then(|result| result.get("content"))
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing watcher tool result text: {response}"))?;
    Ok(serde_json::from_str(text)?)
}

pub(super) fn initialize(client: &mut McpClient) -> Result<(), Box<dyn std::error::Error>> {
    client.send(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }))?;
    Ok(())
}

pub(super) fn open_session(
    client: &mut McpClient,
    id: &str,
    request_id: u64,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let response = client.send(&json!({
        "jsonrpc": "2.0", "id": request_id,
        "method": "tools/call",
        "params": {"name": "watcher_open", "arguments": {
            "assignmentId": id,
            "parent": {"id": "parent"},
            "watcher": {"id": "watcher"},
            "targets": [{"threadId": "target"}],
            "ttlSeconds": 600
        }}
    }))?;
    let payload = tool_payload(&response)?;
    Ok((
        payload["sessionId"].as_str().ok_or("missing session")?.to_owned(),
        payload["parentToken"].as_str().ok_or("missing parent token")?.to_owned(),
        payload["watcherToken"].as_str().ok_or("missing watcher token")?.to_owned(),
    ))
}

#[test]
fn concurrent_reports_and_health_leave_a_restartable_event_log() -> Result<(), String> {
    let state = tempfile::tempdir().map_err(|error| error.to_string())?;
    let mut setup = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut setup).map_err(|error| error.to_string())?;
    let (session, parent_token, watcher_token) =
        open_session(&mut setup, "concurrent-report-health", 2)
            .map_err(|error| error.to_string())?;
    drop(setup);

    let mut observer = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut observer).map_err(|error| error.to_string())?;
    let mut reporter = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut reporter).map_err(|error| error.to_string())?;
    let barrier = Arc::new(Barrier::new(2));
    let reporter_barrier = Arc::clone(&barrier);
    let reporter_session = session.clone();
    let reporter_token = watcher_token.clone();
    let reporter_thread = thread::spawn(move || -> Result<(), String> {
        reporter_barrier.wait();
        for index in 0..32 {
            let response = reporter
                .send(&json!({
                    "jsonrpc": "2.0", "id": index + 10, "method": "tools/call",
                    "params": {"name": "watcher_report", "arguments": {
                        "sessionId": reporter_session, "watcherToken": reporter_token,
                        "target": {"threadId": "target"}, "eventId": format!("event-{index}"),
                        "kind": "gate_ready", "summary": format!("concurrent event {index}")
                    }}
                }))
                .map_err(|error| error.to_string())?;
            if response.get("error").is_some() {
                return Err(format!("report failed: {response}"));
            }
        }
        Ok(())
    });
    barrier.wait();
    for index in 0..40 {
        let response = observer
            .send(&json!({
                "jsonrpc": "2.0", "id": index + 100, "method": "tools/call",
                "params": {"name": "watcher_health", "arguments": {
                    "sessionId": session, "token": parent_token
                }}
            }))
            .map_err(|error| error.to_string())?;
        if response.get("error").is_some() {
            return Err(format!("health failed during report: {response}"));
        }
    }
    reporter_thread
        .join()
        .map_err(|_| "reporter thread panicked".to_owned())??;
    drop(observer);

    let mut restarted = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut restarted).map_err(|error| error.to_string())?;
    let mut cursor = Value::String("0".to_owned());
    let mut observed = 0;
    for index in 0..4 {
        let response = restarted
            .send(&json!({
                "jsonrpc": "2.0", "id": index + 200, "method": "tools/call",
                "params": {"name": "wait_watcher", "arguments": {
                    "sessionId": session, "parentToken": parent_token,
                    "cursor": cursor, "maxReports": 8, "timeoutMs": 1000
                }}
            }))
            .map_err(|error| error.to_string())?;
        let payload = tool_payload(&response).map_err(|error| error.to_string())?;
        if payload["status"] != "event" {
            return Err(format!("restart wait did not return events: {payload}"));
        }
        observed += payload["events"].as_array().map_or(0, Vec::len);
        cursor = payload["nextCursor"].clone();
    }
    if observed != 32 {
        return Err(format!("restart recovered {observed} events instead of 32"));
    }
    Ok(())
}

#[test]
fn session_capacity_is_bounded_and_cancelled_sessions_are_reclaimed() -> Result<(), String> {
    let state = tempfile::tempdir().map_err(|error| error.to_string())?;
    let mut client = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut client).map_err(|error| error.to_string())?;
    let mut sessions = Vec::new();
    for index in 0..128 {
        let (session, parent, _) =
            open_session(&mut client, &format!("capacity-{index}"), index + 2)
                .map_err(|error| error.to_string())?;
        sessions.push((session, parent));
    }
    let overflow = client
        .send(&json!({
            "jsonrpc": "2.0", "id": 500, "method": "tools/call",
            "params": {"name": "watcher_open", "arguments": {
                "assignmentId": "capacity-overflow", "parent": {"id": "parent"},
                "watcher": {"id": "watcher"}, "targets": [{"threadId": "target"}],
                "ttlSeconds": 60
            }}
        }))
        .map_err(|error| error.to_string())?;
    if overflow["error"]["message"] != "watcher session storage is full; wait for expiry before opening another" {
        return Err(format!("capacity guard did not reject the 129th session: {overflow}"));
    }
    let cancelled = client
        .send(&json!({
            "jsonrpc": "2.0", "id": 501, "method": "tools/call",
            "params": {"name": "watcher_cancel", "arguments": {
                "sessionId": sessions[0].0, "parentToken": sessions[0].1
            }}
        }))
        .map_err(|error| error.to_string())?;
    let cancelled_payload = tool_payload(&cancelled)
        .map_err(|error| format!("{error}; response: {cancelled}"))?;
    if cancelled_payload["status"] != "cancelled" {
        return Err(format!("session cancellation failed: {cancelled}"));
    }
    open_session(&mut client, "capacity-after-reclaim", 502)
        .map_err(|error| error.to_string())?;
    Ok(())
}
