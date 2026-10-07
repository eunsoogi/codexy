//! Covers server-derived event identity across retries, reconnects, and state transitions.

use super::*;
use super::watcher_state::{
    initialize, open_session, open_session_with_targets, tool_payload, watcher_client,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

// This payload omits eventId so the runtime derives identity from material state.
fn report_without_id(
    session: &str,
    token: &str,
    target: &str,
    id: u64,
    kind: &str,
    summary: &str,
    evidence: &str,
    observed_at_ms: u64,
) -> Value {
    json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call",
        "params":{"name":"watcher_report", "arguments":{
            "sessionId":session, "watcherToken":token,
            "target":{"threadId":target}, "kind":kind,
            "summary":summary, "evidence":[evidence], "observedAtMs":observed_at_ms
        }}
    })
}

fn wait_page(session: &str, token: &str, cursor: &Value, id: u64) -> Value {
    json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call",
        "params":{"name":"watcher_wait", "arguments":{
            "sessionId":session, "parentToken":token,
            "cursor":cursor, "maxReports":2, "timeoutMs":0
        }}
    })
}

#[test]
fn material_identity_deduplicates_observation_time_and_preserves_transitions() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let (session, parent, watcher) = open_session(&mut client, "material-transitions", 2)?;

    let first = tool_payload(&client.send(&report_without_id(
        &session,
        &watcher,
        "target",
        20,
        "failure",
        "HEAD aaa failed",
        "commit:aaa",
        100,
    ))?)?;
    assert_eq!(first["status"], "accepted");
    assert_eq!(first["cursor"], "1");
    let first_event_id = first["eventId"].as_str().ok_or("missing eventId")?.to_owned();
    drop(client);

    // A restarted client reporting a newer observation time is a retry of the same state.
    let mut restarted = watcher_client(state.path())?;
    initialize(&mut restarted)?;
    let duplicate = tool_payload(&restarted.send(&report_without_id(
        &session,
        &watcher,
        "target",
        21,
        "failure",
        "HEAD aaa failed",
        "commit:aaa",
        200,
    ))?)?;
    assert_eq!(duplicate["status"], "duplicate");
    assert_eq!(duplicate["eventId"], first_event_id);
    assert_eq!(duplicate["cursor"], "1");

    let health = tool_payload(&restarted.send(&json!({
        "jsonrpc":"2.0", "id":22, "method":"tools/call",
        "params":{"name":"watcher_health", "arguments":{
            "sessionId":session, "token":parent
        }}
    }))?)?;
    assert_eq!(health["queueDepth"], 1);
    assert_eq!(health["lastObservationAtMs"], 200);
    assert_eq!(health["lastMaterialEventAtMs"], 100);

    let new_head = tool_payload(&restarted.send(&report_without_id(
        &session,
        &watcher,
        "target",
        23,
        "failure",
        "HEAD bbb failed",
        "commit:bbb",
        300,
    ))?)?;
    assert_eq!(new_head["status"], "accepted");
    assert_eq!(new_head["cursor"], "2");
    let recovery = tool_payload(&restarted.send(&report_without_id(
        &session,
        &watcher,
        "target",
        24,
        "terminal",
        "HEAD bbb recovered",
        "commit:bbb",
        400,
    ))?)?;
    assert_eq!(recovery["status"], "accepted");
    assert_eq!(recovery["cursor"], "3");

    // A failure recurring after recovery is a new transition even when its state repeats.
    let recurrence = tool_payload(&restarted.send(&report_without_id(
        &session,
        &watcher,
        "target",
        25,
        "failure",
        "HEAD bbb failed",
        "commit:bbb",
        500,
    ))?)?;
    assert_eq!(recurrence["status"], "accepted");
    assert_eq!(recurrence["cursor"], "4");
    assert_ne!(recurrence["eventId"], new_head["eventId"]);
    let recurrence_id = recurrence["eventId"].clone();
    let repeated = tool_payload(&restarted.send(&report_without_id(
        &session,
        &watcher,
        "target",
        26,
        "failure",
        "HEAD bbb failed",
        "commit:bbb",
        600,
    ))?)?;
    assert_eq!(repeated["status"], "duplicate");
    assert_eq!(repeated["eventId"], recurrence_id);
    assert_eq!(repeated["cursor"], "4");

    let first_page = tool_payload(&restarted.send(&wait_page(
        &session,
        &parent,
        &json!("0"),
        27,
    ))?)?;
    assert_eq!(first_page["status"], "event");
    assert_eq!(first_page["events"].as_array().ok_or("missing events")?.len(), 2);
    assert_eq!(first_page["nextCursor"], "2");
    let second_page = tool_payload(&restarted.send(&wait_page(
        &session,
        &parent,
        &first_page["nextCursor"],
        28,
    ))?)?;
    assert_eq!(second_page["status"], "event");
    assert_eq!(second_page["events"].as_array().ok_or("missing events")?.len(), 2);
    assert_eq!(second_page["nextCursor"], "4");
    Ok(())
}

#[test]
fn material_identity_is_scoped_to_target_and_lane() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let (session, _, watcher) = open_session_with_targets(
        &mut client,
        "material-subjects",
        2,
        vec![json!({"threadId":"target"}), json!({"threadId":"other"})],
    )?;

    let first_target = tool_payload(&client.send(&report_without_id(
        &session,
        &watcher,
        "target",
        3,
        "gate_ready",
        "same material state",
        "state:ready",
        100,
    ))?)?;
    assert_eq!(first_target["status"], "accepted");
    let other_target = tool_payload(&client.send(&report_without_id(
        &session,
        &watcher,
        "other",
        4,
        "gate_ready",
        "same material state",
        "state:ready",
        100,
    ))?)?;
    assert_eq!(other_target["status"], "accepted");
    assert_ne!(first_target["eventId"], other_target["eventId"]);

    let target_retry = tool_payload(&client.send(&report_without_id(
        &session,
        &watcher,
        "target",
        5,
        "gate_ready",
        "same material state",
        "state:ready",
        200,
    ))?)?;
    assert_eq!(target_retry["status"], "duplicate");
    assert_eq!(target_retry["cursor"], "1");

    let (other_lane, _, other_watcher) = open_session(&mut client, "material-lane-two", 6)?;
    let other_lane_event = tool_payload(&client.send(&report_without_id(
        &other_lane,
        &other_watcher,
        "target",
        7,
        "gate_ready",
        "same material state",
        "state:ready",
        100,
    ))?)?;
    assert_eq!(other_lane_event["status"], "accepted");
    assert_eq!(other_lane_event["cursor"], "1");
    assert_ne!(first_target["eventId"], other_lane_event["eventId"]);
    Ok(())
}
