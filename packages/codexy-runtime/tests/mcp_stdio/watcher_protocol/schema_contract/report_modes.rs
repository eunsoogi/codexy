use super::super::*;
use super::{assert_error_contains, assert_status, call_report, watcher_session};

#[test]
fn parser_matches_report_enums_and_health_field_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let (_state, mut client, session, watcher_token, target) = watcher_session()?;
    for (index, kind) in [
        "terminal",
        "failure",
        "drift",
        "missing_delivery",
        "gate_ready",
        "health",
        "unavailable",
    ]
    .into_iter()
    .enumerate()
    {
        let response = call_report(
            &mut client,
            10 + index as u64,
            &session,
            &watcher_token,
            json!({"event":{
                "target":target,
                "eventId":format!("kind-{kind}"),
                "kind":kind,
                "summary":"supported kind"
            }}),
        )?;
        assert_status(&response, "accepted")?;
    }

    let unsupported_kind = call_report(
        &mut client,
        20,
        &session,
        &watcher_token,
        json!({"event":{
            "target":target,
            "eventId":"unsupported-kind",
            "kind":"not-supported",
            "summary":"invalid kind"
        }}),
    )?;
    assert_error_contains(&unsupported_kind, "event kind is not supported");

    for (index, state) in [
        "starting",
        "running",
        "idle",
        "error",
        "stopped",
        "unavailable",
    ]
    .into_iter()
    .enumerate()
    {
        let response = call_report(
            &mut client,
            30 + index as u64,
            &session,
            &watcher_token,
            json!({"target":target,"watcherState":state}),
        )?;
        assert_status(&response, "health_updated")?;
    }

    let unsupported_state = call_report(
        &mut client,
        36,
        &session,
        &watcher_token,
        json!({"target":target,"watcherState":"sleeping"}),
    )?;
    assert_error_contains(&unsupported_state, "watcher state is not supported");

    let error_128 = call_report(
        &mut client,
        24,
        &session,
        &watcher_token,
        json!({"target":target,"lastError":"x".repeat(128),"watcherState":"error"}),
    )?;
    assert_status(&error_128, "health_updated")?;
    let error_129 = call_report(
        &mut client,
        25,
        &session,
        &watcher_token,
        json!({"target":target,"lastError":"x".repeat(129),"watcherState":"error"}),
    )?;
    assert_error_contains(&error_129, "lastError is invalid");

    let negative_timestamp = call_report(
        &mut client,
        37,
        &session,
        &watcher_token,
        json!({"target":target,"observedAtMs":-1}),
    )?;
    assert_error_contains(&negative_timestamp, "observedAtMs must be an integer");
    Ok(())
}

#[test]
fn requested_health_event_is_delivered_to_parent_waiters()
-> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut client = super::super::watcher_client(state.path())?;
    client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
    }))?;
    let opened = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"watcher_open","arguments":{
            "assignmentId":"requested-health",
            "parent":{"id":"parent-task"},
            "watcher":{"id":"native-watcher"},
            "targets":[{"threadId":"target-thread"}],
            "ttlSeconds":60
        }}
    }))?;
    let opened = super::super::tool_payload(&opened)?;
    let session = opened["sessionId"].as_str().ok_or("session id")?;
    let watcher_token = opened["watcherToken"].as_str().ok_or("watcher token")?;
    let parent_token = opened["parentToken"].as_str().ok_or("parent token")?;
    let target = json!({"threadId":"target-thread"});

    let metadata = call_report(
        &mut client,
        3,
        session,
        watcher_token,
        json!({"target":target,"watcherState":"running"}),
    )?;
    assert_status(&metadata, "health_updated")?;
    let waiting = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":0
        }}
    }))?;
    assert_eq!(super::super::tool_payload(&waiting)?["status"], "timeout");

    let report = call_report(
        &mut client,
        5,
        session,
        watcher_token,
        json!({"event":{
            "target":target,"eventId":"requested-health-event",
            "kind":"health","summary":"Watcher is running"
        },"watcherState":"running"}),
    )?;
    assert!(
        report["error"]["message"].is_null(),
        "requested status report was rejected: {}",
        report["error"]["message"]
    );
    assert_status(&report, "accepted")?;
    let delivered = client.send(&json!({
        "jsonrpc":"2.0","id":6,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":0
        }}
    }))?;
    let delivered = super::super::tool_payload(&delivered)?;
    assert_eq!(delivered["status"], "event");
    assert_eq!(delivered["events"][0]["kind"], "health");
    assert_eq!(delivered["events"][0]["summary"], "Watcher is running");
    Ok(())
}

#[test]
fn parser_rejects_path_separators_in_assignment_ids() -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut client = super::super::watcher_client(state.path())?;
    client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
    }))?;

    for (request_id, assignment_id) in ["unsafe/id", r"unsafe\id"].into_iter().enumerate() {
        let response = client.send(&json!({
            "jsonrpc":"2.0","id":request_id + 2,"method":"tools/call",
            "params":{"name":"watcher_open","arguments":{
                "assignmentId":assignment_id,
                "parent":{"id":"parent-task"},
                "watcher":{"id":"native-watcher"},
                "targets":[{"threadId":"target-thread"}]
            }}
        }))?;
        super::assert_error_contains(&response, "watcher assignmentId is invalid");
    }
    Ok(())
}
