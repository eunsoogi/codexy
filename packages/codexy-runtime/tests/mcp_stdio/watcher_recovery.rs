use super::*;
use super::watcher_state::{initialize, open_session, tool_payload, watcher_client};

fn event_path(state: &Path, session: &str) -> PathBuf {
    state
        .join("codexy-watcher")
        .join(session)
        .join("events.jsonl")
}

fn reporter_report(
    reporter: &mut McpClient,
    session: &str,
    watcher_token: &str,
    request_id: u64,
    event_id: &str,
) -> Result<Value, String> {
    let response = reporter
        .send(&json!({
            "jsonrpc": "2.0", "id": request_id, "method": "tools/call",
            "params": {"name": "watcher_report", "arguments": {
                "sessionId": session, "watcherToken": watcher_token,
                "target": {"threadId": "target"}, "eventId": event_id,
                "kind": "gate_ready", "summary": event_id, "observedAtMs": 1_000
            }}
        }))
        .map_err(|error| error.to_string())?;
    tool_payload(&response).map_err(|error| format!("{error}; response: {response}"))
}

fn wait_all(
    reader: &mut McpClient,
    session: &str,
    parent_token: &str,
    request_id: u64,
) -> Result<Value, String> {
    let response = reader
        .send(&json!({
            "jsonrpc": "2.0", "id": request_id, "method": "tools/call",
            "params": {"name": "wait_watcher", "arguments": {
                "sessionId": session, "parentToken": parent_token,
                "cursor": "0", "maxReports": 8, "timeoutMs": 1000
            }}
        }))
        .map_err(|error| error.to_string())?;
    tool_payload(&response).map_err(|error| format!("{error}; response: {response}"))
}

#[test]
fn interrupted_utf8_event_tail_is_recovered_before_the_next_report() -> Result<(), String> {
    let state = tempfile::tempdir().map_err(|error| error.to_string())?;
    let mut setup = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut setup).map_err(|error| error.to_string())?;
    let (session, parent_token, watcher_token) =
        open_session(&mut setup, "interrupted-tail", 2).map_err(|error| error.to_string())?;
    let first = reporter_report(&mut setup, &session, &watcher_token, 3, "first")?;
    if first["status"] != "accepted" || first["cursor"] != "1" {
        return Err(format!("initial report was invalid: {first}"));
    }
    drop(setup);

    let path = event_path(state.path(), &session);
    let mut events = std::fs::read(&path).map_err(|error| error.to_string())?;
    events.extend_from_slice(
        br#"{"eventId":"interrupted","sequence":2,"kind":"gate_ready","target":{"threadId":"target"},"summary":"#,
    );
    events.extend_from_slice(&"완료".as_bytes()[..1]);
    std::fs::write(&path, events).map_err(|error| error.to_string())?;

    let mut reporter = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut reporter).map_err(|error| error.to_string())?;
    let recovered = reporter_report(&mut reporter, &session, &watcher_token, 4, "recovered")?;
    if recovered["status"] != "accepted" || recovered["cursor"] != "2" {
        return Err(format!("recovered report was not accepted: {recovered}"));
    }
    let duplicate = reporter_report(&mut reporter, &session, &watcher_token, 5, "first")?;
    if duplicate["status"] != "duplicate" || duplicate["cursor"] != "1" {
        return Err(format!("previous event was not deduplicated: {duplicate}"));
    }

    let mut reader = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut reader).map_err(|error| error.to_string())?;
    let payload = wait_all(&mut reader, &session, &parent_token, 6)?;
    let events = payload["events"]
        .as_array()
        .ok_or_else(|| format!("recovered wait omitted events: {payload}"))?;
    if payload["status"] != "event"
        || events.len() != 2
        || events[0]["summary"] != "first"
        || events[1]["summary"] != "recovered"
    {
        return Err(format!("recovered wait was invalid: {payload}"));
    }
    Ok(())
}

#[test]
fn complete_event_without_final_newline_is_preserved_before_append() -> Result<(), String> {
    let state = tempfile::tempdir().map_err(|error| error.to_string())?;
    let mut setup = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut setup).map_err(|error| error.to_string())?;
    let (session, parent_token, watcher_token) =
        open_session(&mut setup, "complete-tail", 2).map_err(|error| error.to_string())?;
    let first = reporter_report(&mut setup, &session, &watcher_token, 3, "first")?;
    if first["status"] != "accepted" {
        return Err(format!("initial report was invalid: {first}"));
    }
    let path = event_path(state.path(), &session);
    let mut events = std::fs::read(&path).map_err(|error| error.to_string())?;
    if events.pop() != Some(b'\n') {
        return Err("event log did not end with a newline".to_owned());
    }
    std::fs::write(&path, events).map_err(|error| error.to_string())?;
    drop(setup);

    let mut reporter = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut reporter).map_err(|error| error.to_string())?;
    let appended = reporter_report(&mut reporter, &session, &watcher_token, 4, "appended")?;
    if appended["status"] != "accepted" || appended["cursor"] != "2" {
        return Err(format!("append after complete tail was invalid: {appended}"));
    }
    let mut reader = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut reader).map_err(|error| error.to_string())?;
    let payload = wait_all(&mut reader, &session, &parent_token, 5)?;
    let events = payload["events"]
        .as_array()
        .ok_or_else(|| format!("complete-tail wait omitted events: {payload}"))?;
    if payload["status"] != "event"
        || events.len() != 2
        || events[0]["summary"] != "first"
        || events[1]["summary"] != "appended"
    {
        return Err(format!("complete-tail wait was invalid: {payload}"));
    }
    Ok(())
}

#[test]
fn incomplete_session_with_known_atomic_temp_is_reclaimed() -> Result<(), String> {
    let state = tempfile::tempdir().map_err(|error| error.to_string())?;
    let orphan = state.path().join("codexy-watcher/orphan");
    std::fs::create_dir_all(&orphan).map_err(|error| error.to_string())?;
    let temporary = orphan.join(format!(".session.json.tmp-{}", "a".repeat(24)));
    std::fs::write(&temporary, b"partial session").map_err(|error| error.to_string())?;

    let mut client = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut client).map_err(|error| error.to_string())?;
    let opened = open_session(&mut client, "reclaim-known-temp", 2)
        .map_err(|error| error.to_string())?;
    if temporary.exists() || orphan.exists() || opened.0.is_empty() {
        return Err("known atomic temporary was not reclaimed".to_owned());
    }
    Ok(())
}

#[test]
fn incomplete_session_with_unknown_file_is_preserved_and_rejected() -> Result<(), String> {
    let state = tempfile::tempdir().map_err(|error| error.to_string())?;
    let orphan = state.path().join("codexy-watcher/orphan");
    std::fs::create_dir_all(&orphan).map_err(|error| error.to_string())?;
    let lock = orphan.join("state.lock");
    std::fs::write(&lock, b"user-owned lock").map_err(|error| error.to_string())?;

    let mut client = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut client).map_err(|error| error.to_string())?;
    let response = client
        .send(&serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"watcher_open","arguments":{
                "assignmentId":"reject-unknown","parent":{"id":"parent"},
                "watcher":{"id":"watcher"},"targets":[{"threadId":"target"}],
                "ttlSeconds":60
            }}
        }))
        .map_err(|error| error.to_string())?;
    if response["error"]["message"] != "watcher session directory is incomplete" {
        return Err(format!("unknown incomplete session was not rejected: {response}"));
    }
    if !lock.exists() {
        return Err("unknown incomplete-session file was removed".to_owned());
    }
    Ok(())
}
