use super::*;

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

// The default stays below the observed transport deadline while callers can request the full cap.
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
        .and_then(|tools| tools.iter().find(|tool| tool["name"] == "watcher_wait"))
        .ok_or("watcher_wait schema is missing")?;
    let timeout = &wait["inputSchema"]["properties"]["timeoutMs"];
    assert_eq!(timeout["default"], 295_000);
    assert_eq!(timeout["maximum"], LONG_WAIT_MS);
    assert_ne!(timeout["default"], timeout["maximum"]);
    let description = wait["description"]
        .as_str()
        .ok_or("watcher_wait description is missing")?;
    assert!(description.contains("295000 ms"));
    assert!(description.contains("status=expired"));
    assert!(description.contains("empty events"));
    assert!(description.contains("unchanged nextCursor"));
    assert!(description.contains("does not consume or modify the durable event log"));
    assert!(description.contains("notifications/cancelled"));
    assert!(description.contains("watcher_cancel is separate"));
    assert!(description.contains("sessionId, decimal-string nextCursor, unchanged events"));
    assert!(description.contains("lastObservationAtMs, and lastError"));
    assert!(description.contains("cancellationReason=request_cancelled"));
    assert!(description.contains("cancellationReason=session_cancelled"));
    assert!(description.contains("A caller MUST call watcher_health only when detailed diagnostics are explicitly needed"));
    Ok(())
}
