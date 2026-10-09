use super::*;
use super::super::watcher_state::{
    initialize, open_session, open_session_with_targets, tool_payload, watcher_client,
};
use std::fs;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn wait_page(session: &str, token: &str, cursor: &Value, id: u64) -> Value {
    json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call",
        "params":{"name":"watcher_wait", "arguments":{
            "sessionId":session, "parentToken":token,
            "cursor":cursor, "maxReports":2, "timeoutMs":0
        }}
    })
}

pub(crate) fn assert_wait_health_fields(wait: &Value) {
    let mut keys = wait["health"]
        .as_object()
        .expect("wait health must be an object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "lastError".to_owned(),
            "lastObservationAtMs".to_owned(),
            "status".to_owned(),
            "watcherState".to_owned()
        ]
    );
}

fn measure_empty_wait_bytes(target_count: usize) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let targets = (0..target_count)
        .map(|index| json!({"threadId":format!("target-{}", index + 1)}))
        .collect();
    let assignment = format!("measure-{target_count}");
    let (session, parent, _) =
        open_session_with_targets(&mut client, &assignment, 10 + target_count as u64, targets)?;

    // Fixed persisted values make old and new stdio payloads directly comparable.
    let root = state.path().join("codexy-watcher");
    let session_dir = root.join(&session);
    let fixed_session = assignment;
    let fixed_dir = root.join(&fixed_session);
    fs::rename(&session_dir, &fixed_dir)?;
    let session_path = fixed_dir.join("session.json");
    let mut session_record: Value = serde_json::from_slice(&fs::read(&session_path)?)?;
    session_record["sessionId"] = json!(fixed_session);
    session_record["expiresAtMs"] = json!(2_000_000_000_000_u64);
    fs::write(&session_path, serde_json::to_vec(&session_record)?)?;
    fs::write(
        fixed_dir.join("health.json"),
        serde_json::to_vec(&json!({
            "lastObservationAtMs": null,
            "lastMaterialEventAtMs": null,
            "watcherState": "unknown",
            "lastError": null
        }))?,
    )?;

    let response = client.send(&wait_page(
        &fixed_session,
        &parent,
        &json!("0"),
        100 + target_count as u64,
    ))?;
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("missing watcher result text")?;
    let payload_bytes = text.as_bytes().len();
    let content_bytes = serde_json::to_vec(&response["result"]["content"])?.len();
    eprintln!(
        "watcher_wait_bytes targets={target_count} payload_json_utf8={payload_bytes} mcp_content_utf8={content_bytes}"
    );
    Ok((payload_bytes, content_bytes))
}

#[test]
fn wait_uses_small_health_while_watcher_health_keeps_detailed_metadata() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let (session, parent, _) = open_session(&mut client, "response-shape", 2)?;

    let wait = tool_payload(&client.send(&wait_page(&session, &parent, &json!("0"), 3))?)?;
    assert_eq!(wait["status"], "timeout");
    assert_eq!(wait["sessionId"], session);
    assert_eq!(wait["nextCursor"], "0");
    assert_eq!(wait["events"], json!([]));
    assert_eq!(wait.as_object().expect("wait result object").len(), 5);
    assert_wait_health_fields(&wait);
    assert_eq!(wait["health"]["status"], "active");
    assert_eq!(wait["health"]["watcherState"], "starting");
    assert_eq!(wait["health"]["lastObservationAtMs"], Value::Null);
    assert_eq!(wait["health"]["lastError"], Value::Null);

    let health = tool_payload(&client.send(&json!({
        "jsonrpc":"2.0", "id":4, "method":"tools/call",
        "params":{"name":"watcher_health", "arguments":{
            "sessionId":session, "token":parent
        }}
    }))?)?;
    assert_eq!(health["assignmentId"], "response-shape");
    assert_eq!(health["parent"], json!({"id":"parent"}));
    assert_eq!(health["watcher"], json!({"id":"watcher"}));
    assert_eq!(health["targets"], json!([{"threadId":"target"}]));
    assert_eq!(health["generation"], 1);
    assert_eq!(health["queueDepth"], 0);
    assert_eq!(health["transport"], "filesystem-queue");
    assert_eq!(health["transportConnected"], true);
    assert_eq!(health["nativeStatus"], "unverified");
    Ok(())
}

#[test]
fn wait_health_preserves_unknown_state_null_time_and_last_error() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let (session, parent, _) = open_session(&mut client, "unknown-health", 2)?;
    let health_path = state
        .path()
        .join("codexy-watcher")
        .join(&session)
        .join("health.json");
    fs::write(
        health_path,
        serde_json::to_vec(&json!({
            "lastObservationAtMs": null,
            "lastMaterialEventAtMs": null,
            "watcherState": "unknown",
            "lastError": "observation unavailable"
        }))?,
    )?;

    let wait = tool_payload(&client.send(&wait_page(&session, &parent, &json!("0"), 3))?)?;
    assert_wait_health_fields(&wait);
    assert_eq!(wait["health"]["status"], "active");
    assert_eq!(wait["health"]["watcherState"], "unknown");
    assert_eq!(wait["health"]["lastObservationAtMs"], Value::Null);
    assert_eq!(wait["health"]["lastError"], "observation unavailable");
    Ok(())
}

#[test]
fn empty_wait_response_sizes_are_measured_for_one_and_eight_targets() -> TestResult {
    for target_count in [1, 8] {
        measure_empty_wait_bytes(target_count)?;
    }
    Ok(())
}
