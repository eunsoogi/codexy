use super::*;
use super::watcher_state::{initialize, open_session, tool_payload, watcher_client};
use std::fs::{self, OpenOptions};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn report(session: &str, token: &str, id: u64) -> Value {
    json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call",
        "params":{"name":"watcher_report", "arguments":{
            "sessionId":session, "watcherToken":token,
            "target":{"threadId":"target"}, "eventId":format!("input-{id}"),
            "kind":"gate_ready", "summary":format!("message-{id}"),
            "evidence":[format!("evidence-{id}")], "observedAtMs":id
        }}
    })
}

fn wait_page(session: &str, token: &str, cursor: &Value, id: u64) -> Value {
    json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call",
        "params":{"name":"wait_watcher", "arguments":{
            "sessionId":session, "parentToken":token,
            "cursor":cursor, "maxReports":2, "timeoutMs":0
        }}
    })
}

#[test]
fn approved_events_and_health_survive_restart_with_exact_pagination() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut writer = watcher_client(state.path())?;
    initialize(&mut writer)?;
    let (session, parent, watcher) = open_session(&mut writer, "exact-restart", 2)?;
    let mut observer = watcher_client(state.path())?;
    initialize(&mut observer)?;
    let mut approved = Vec::new();
    for id in 3..6 {
        let response = tool_payload(&writer.send(&report(&session, &watcher, id))?)?;
        assert_eq!(response["status"], "accepted");
        assert_eq!(response["cursor"], (id - 2).to_string());
        approved.push(response["eventId"].as_str().ok_or("missing eventId")?.to_owned());
        let health = tool_payload(&observer.send(&json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call",
            "params":{"name":"watcher_health", "arguments":{
                "sessionId":session, "token":parent
            }}
        }))?)?;
        assert_eq!(health["sessionId"], session);
        assert_eq!(health["assignmentId"], "exact-restart");
        assert_eq!(health["status"], "active");
        assert_eq!(health["queueDepth"], id - 2);
        assert_eq!(health["lastMaterialEventAtMs"], id);
    }
    assert_ne!(approved[0], approved[1]);
    assert_ne!(approved[0], approved[2]);
    assert_ne!(approved[1], approved[2]);
    drop(writer);
    drop(observer);

    let mut restarted = watcher_client(state.path())?;
    initialize(&mut restarted)?;
    let mut cursor = json!("0");
    let mut events = Vec::new();
    for (id, length, next) in [(10, 2, "2"), (11, 1, "3")] {
        let page = tool_payload(&restarted.send(&wait_page(&session, &parent, &cursor, id))?)?;
        assert_eq!(page["status"], "event");
        let page_events = page["events"].as_array().ok_or("missing events")?;
        assert_eq!(page_events.len(), length);
        assert_eq!(page["nextCursor"], next);
        events.extend(page_events.iter().cloned());
        cursor = page["nextCursor"].clone();
    }
    assert_eq!(events.len(), approved.len());
    for (index, event) in events.iter().enumerate() {
        let input_id = index + 3;
        assert_eq!(event["eventId"], approved[index]);
        assert_eq!(event["sequence"], index + 1);
        assert_eq!(event["summary"], format!("message-{input_id}"));
        assert_eq!(event["target"], json!({"threadId":"target"}));
        assert_eq!(event["kind"], "gate_ready");
        assert_eq!(event["evidence"], json!([format!("evidence-{input_id}")]));
        assert_eq!(event["observedAtMs"], input_id);
    }
    let empty = tool_payload(&restarted.send(&wait_page(&session, &parent, &cursor, 12))?)?;
    assert_eq!(empty["status"], "timeout");
    assert_eq!(empty["events"], json!([]));
    assert_eq!(empty["nextCursor"], "3");
    Ok(())
}

#[test]
fn held_transition_lock_rejects_writes_and_same_process_advances_after_release() -> TestResult {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let (session, parent, watcher) = open_session(&mut client, "controlled-contention", 2)?;
    let root = state.path().join("codexy-watcher");
    let directory = root.join(&session);
    let before_session = fs::read(directory.join("session.json"))?;
    let before_health = fs::read(directory.join("health.json"))?;
    let lock_path = root.join(".reclaim.lock");
    let lock = OpenOptions::new().read(true).write(true).open(&lock_path)?;
    lock.lock()?;
    // The lock remains owned until the child returns, so this is an actual
    // acquisition failure, independent of process scheduling or an initial barrier.
    let rejected = client.send(&report(&session, &watcher, 3))?;
    assert_eq!(rejected["error"]["code"], -32000);
    assert_eq!(rejected["error"]["message"], format!("watcher state lock is busy: {}", lock_path.display()));
    assert_eq!(fs::read(directory.join("session.json"))?, before_session);
    assert_eq!(fs::read(directory.join("health.json"))?, before_health);
    assert!(!directory.join("events.jsonl").exists());
    lock.unlock()?;
    drop(lock);

    let accepted = tool_payload(&client.send(&report(&session, &watcher, 4))?)?;
    assert_eq!(accepted["status"], "accepted");
    assert_eq!(accepted["cursor"], "1");
    let page = tool_payload(&client.send(&wait_page(&session, &parent, &json!("0"), 5))?)?;
    let events = page["events"].as_array().ok_or("missing events")?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["eventId"], accepted["eventId"]);
    assert_eq!(events[0]["summary"], "message-4");
    assert_eq!(events[0]["sequence"], 1);
    assert_eq!(page["nextCursor"], "1");
    Ok(())
}
