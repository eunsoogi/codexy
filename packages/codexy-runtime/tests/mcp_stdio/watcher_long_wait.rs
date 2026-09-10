use super::*;
use super::watcher_state::{initialize, open_session, tool_payload, watcher_client};
use std::thread;
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LONG_WAIT_MS: u64 = 3_600_000;

fn open_session_with_ttl(
    client: &mut McpClient,
    assignment: &str,
    request_id: u64,
    ttl_seconds: u64,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let response = client.send(&json!({
        "jsonrpc": "2.0", "id": request_id, "method": "tools/call",
        "params": {"name": "watcher_open", "arguments": {
            "assignmentId": assignment,
            "parent": {"id": "parent"},
            "watcher": {"id": "watcher"},
            "targets": [{"threadId": "target"}],
            "ttlSeconds": ttl_seconds
        }}
    }))?;
    let payload = tool_payload(&response)?;
    Ok((
        payload["sessionId"].as_str().ok_or("missing session")?.to_owned(),
        payload["parentToken"]
            .as_str()
            .ok_or("missing parent token")?
            .to_owned(),
        payload["watcherToken"]
            .as_str()
            .ok_or("missing watcher token")?
            .to_owned(),
    ))
}

fn wait_request(
    request_id: u64,
    session: &str,
    parent_token: &str,
    cursor: &str,
    timeout_ms: u64,
) -> Value {
    json!({
        "jsonrpc": "2.0", "id": request_id, "method": "tools/call",
        "params": {"name": "wait_watcher", "arguments": {
            "sessionId": session, "parentToken": parent_token,
            "cursor": cursor, "timeoutMs": timeout_ms
        }}
    })
}

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
fn long_wait_bound_rejects_only_values_above_one_hour() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let (session, parent_token, _) = open_session(&mut client, "long-bound", 2)?;
    let response = client.send(&wait_request(
        3,
        &session,
        &parent_token,
        "0",
        LONG_WAIT_MS + 1,
    ))?;
    assert_eq!(
        response["error"]["message"],
        "watcher timeoutMs must be at most 3600000"
    );
    Ok(())
}

#[test]
fn long_wait_returns_a_material_event_without_timeout_polling() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, watcher_token) = open_session(&mut setup, "long-event", 2)?;
    drop(setup);

    let mut reader = watcher_client(state.path())?;
    initialize(&mut reader)?;
    reader.send_without_read(&wait_request(
        4,
        &session,
        &parent_token,
        "0",
        LONG_WAIT_MS,
    ))?;
    let mut observer = watcher_client(state.path())?;
    initialize(&mut observer)?;
    waiting_until_true(&mut observer, &session, &parent_token)?;

    let mut reporter = watcher_client(state.path())?;
    initialize(&mut reporter)?;
    let started = Instant::now();
    let reported = reporter.send(&json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": {"name": "watcher_report", "arguments": {
            "sessionId": session, "watcherToken": watcher_token,
            "target": {"threadId": "target"}, "eventId": "long-event",
            "kind": "gate_ready", "summary": "material event"
        }}
    }))?;
    assert_eq!(tool_payload(&reported)?["status"], "accepted");
    let waited = tool_payload(&reader.read_frame()?)?;
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(waited["status"], "event");
    assert_eq!(waited["nextCursor"], "1");
    assert_eq!(waited["events"].as_array().ok_or("missing events")?.len(), 1);
    Ok(())
}

#[test]
fn watcher_cancel_releases_a_long_wait_without_changing_the_cursor() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, _) = open_session(&mut setup, "long-cancel", 2)?;
    drop(setup);

    let mut reader = watcher_client(state.path())?;
    initialize(&mut reader)?;
    reader.send_without_read(&wait_request(
        4,
        &session,
        &parent_token,
        "0",
        LONG_WAIT_MS,
    ))?;
    let mut observer = watcher_client(state.path())?;
    initialize(&mut observer)?;
    waiting_until_true(&mut observer, &session, &parent_token)?;

    let mut canceller = watcher_client(state.path())?;
    initialize(&mut canceller)?;
    let started = Instant::now();
    let cancelled = canceller.send(&json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": {"name": "watcher_cancel", "arguments": {
            "sessionId": session, "parentToken": parent_token
        }}
    }))?;
    assert_eq!(tool_payload(&cancelled)?["status"], "cancelled");
    let waited = tool_payload(&reader.read_frame()?)?;
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(waited["status"], "cancelled");
    assert_eq!(waited["nextCursor"], "0");
    assert_eq!(waited["events"], json!([]));
    Ok(())
}

#[test]
fn ttl_expiry_preserves_queued_events_on_disk() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, watcher_token) =
        open_session_with_ttl(&mut setup, "long-ttl", 2, 1)?;
    let reported = setup.send(&json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": {"name": "watcher_report", "arguments": {
            "sessionId": session, "watcherToken": watcher_token,
            "target": {"threadId": "target"}, "eventId": "before-expiry",
            "kind": "gate_ready", "summary": "queued before expiry"
        }}
    }))?;
    assert_eq!(tool_payload(&reported)?["status"], "accepted");
    drop(setup);

    let mut reader = watcher_client(state.path())?;
    initialize(&mut reader)?;
    let started = Instant::now();
    let expired = tool_payload(&reader.send(&wait_request(
        4,
        &session,
        &parent_token,
        "1",
        LONG_WAIT_MS,
    ))?)?;
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(expired["status"], "expired");
    assert_eq!(expired["nextCursor"], "1");
    assert_eq!(expired["events"], json!([]));

    let mut reconnect = watcher_client(state.path())?;
    initialize(&mut reconnect)?;
    let still_expired = tool_payload(&reconnect.send(&wait_request(
        5,
        &session,
        &parent_token,
        "0",
        0,
    ))?)?;
    assert_eq!(still_expired["status"], "expired");
    assert_eq!(still_expired["nextCursor"], "0");
    assert_eq!(still_expired["events"], json!([]));
    assert_eq!(still_expired["health"]["queueDepth"], 1);
    Ok(())
}

#[test]
fn wait_schema_documents_the_long_poll_bounds() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let listed = client.send(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
    }))?;
    let wait = listed["result"]["tools"]
        .as_array()
        .and_then(|tools| tools.iter().find(|tool| tool["name"] == "wait_watcher"))
        .ok_or("wait_watcher schema is missing")?;
    let timeout = &wait["inputSchema"]["properties"]["timeoutMs"];
    assert_eq!(timeout["default"], 600_000);
    assert_eq!(timeout["maximum"], 3_600_000);
    let description = wait["description"]
        .as_str()
        .ok_or("wait_watcher description is missing")?;
    assert!(description.contains("status=expired"));
    assert!(description.contains("empty events"));
    assert!(description.contains("unchanged nextCursor"));
    assert!(description.contains("durable queue preserved"));
    Ok(())
}
