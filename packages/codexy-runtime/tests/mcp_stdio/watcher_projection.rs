//! Verifies the public wait-event projection and its response-size effect.

use super::*;
use super::super::watcher_state::{initialize, open_session, tool_payload, watcher_client};
use std::fs;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// Captured at integrated #1306 base 6a0edb09 before the response projection change.
const PRE_PROJECTION_EVENT_WAIT_BYTES: [(usize, usize); 2] = [(514, 601), (2852, 3163)];

fn measure_event_wait_bytes(
    report_count: usize,
) -> Result<(Value, Vec<Value>, usize, usize), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let assignment = format!("measure-event-batch-{report_count}");
    let (session, parent, watcher) = open_session(&mut client, &assignment, 10)?;

    let root = state.path().join("codexy-watcher");
    let session_dir = root.join(&session);
    let fixed_session = assignment;
    let fixed_dir = root.join(&fixed_session);
    // Event identity includes the session id, so fix it before reports to make byte inputs stable.
    fs::rename(&session_dir, &fixed_dir)?;
    let session_path = fixed_dir.join("session.json");
    let mut session_record: Value = serde_json::from_slice(&fs::read(&session_path)?)?;
    session_record["sessionId"] = json!(fixed_session);
    session_record["expiresAtMs"] = json!(2_000_000_000_000_u64);
    fs::write(&session_path, serde_json::to_vec(&session_record)?)?;

    let mut expected = Vec::new();
    for index in 1..=report_count {
        let target = json!({"threadId":"target"});
        let summary = format!("검증 완료 🚀 {index}");
        let evidence = json!(["commit:abc123", format!("log:기록-{index}")]);
        let observed_at_ms = 1_700_000_000_000_u64 + index as u64;
        let accepted = tool_payload(&client.send(&json!({
            "jsonrpc":"2.0", "id":200 + index, "method":"tools/call",
            "params":{"name":"watcher_report", "arguments":{
                "sessionId":fixed_session, "watcherToken":watcher,
                "target":target.clone(), "eventId":format!("measure-{index}"),
                "kind":"gate_ready", "summary":summary, "evidence":evidence,
                "observedAtMs":observed_at_ms
            }}
        }))?)?;
        assert_eq!(accepted["status"], "accepted");
        expected.push(json!({
            "eventId":accepted["eventId"],
            "sequence":index,
            "kind":"gate_ready",
            "target":target,
            "summary":format!("검증 완료 🚀 {index}"),
            "observedAtMs":observed_at_ms,
            "evidence":["commit:abc123", format!("log:기록-{index}")]
        }));
    }

    fs::write(
        fixed_dir.join("health.json"),
        serde_json::to_vec(&json!({
            "lastObservationAtMs": null,
            "lastMaterialEventAtMs": null,
            "watcherState": "unknown",
            "lastError": null
        }))?,
    )?;
    let response = client.send(&json!({
        "jsonrpc":"2.0", "id":300 + report_count, "method":"tools/call",
        "params":{"name":"watcher_wait", "arguments":{
            "sessionId":fixed_session, "parentToken":parent,
            "cursor":"0", "maxReports":report_count, "timeoutMs":0
        }}
    }))?;
    let page = tool_payload(&response)?;
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("missing watcher result text")?;
    let payload_bytes = text.as_bytes().len();
    let content_bytes = serde_json::to_vec(&response["result"]["content"])?.len();
    Ok((page, expected, payload_bytes, content_bytes))
}

#[test]
fn event_wait_projection_preserves_full_values_and_reduces_payloads() -> TestResult {
    let mut measurements = Vec::new();
    for (index, report_count) in [1, 8].into_iter().enumerate() {
        let (page, expected, payload_bytes, content_bytes) =
            measure_event_wait_bytes(report_count)?;
        let (baseline_payload_bytes, baseline_content_bytes) =
            PRE_PROJECTION_EVENT_WAIT_BYTES[index];
        eprintln!(
            "watcher_wait_event_bytes reports={report_count} baseline_payload_json_utf8={baseline_payload_bytes} payload_json_utf8={payload_bytes} baseline_mcp_content_utf8={baseline_content_bytes} mcp_content_utf8={content_bytes}"
        );
        assert!(payload_bytes < baseline_payload_bytes);
        assert!(content_bytes < baseline_content_bytes);
        assert_eq!(page["status"], "event");
        assert_eq!(page["events"].as_array().map(Vec::len), Some(report_count));
        measurements.push((page, expected));
    }

    for (page, expected) in measurements {
        let events = page["events"].as_array().expect("wait event array");
        for (event, expected) in events.iter().zip(expected) {
            super::watcher_response::assert_public_event(event, &expected);
        }
    }
    Ok(())
}
