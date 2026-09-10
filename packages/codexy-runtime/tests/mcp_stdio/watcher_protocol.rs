use super::*;
use std::thread;
use std::time::Duration;
use std::process::Stdio;

fn watcher_client(
    state_dir: &std::path::Path,
) -> Result<McpClient, Box<dyn std::error::Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-watcher"));
    command
        .env("CODEXY_WATCHER_STATE_DIR", state_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    McpClient::spawn_command(command)
}

fn tool_payload(response: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(
        response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("missing watcher tool result text")?,
    )?)
}

#[test]
fn watcher_wait_is_released_by_mcp_cancellation_without_consuming_later_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    let init = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
    }))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-watcher");
    let list = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/list","params":{}
    }))?;
    let names = list["result"]["tools"]
        .as_array()
        .ok_or("watcher tools must be an array")?;
    let canonical = names
        .iter()
        .find(|tool| tool["name"] == "watcher_wait")
        .ok_or("canonical watcher_wait tool is missing")?;
    let compatibility = names
        .iter()
        .find(|tool| tool["name"] == "wait_watcher")
        .ok_or("wait_watcher compatibility tool is missing")?;
    assert_eq!(canonical["inputSchema"], compatibility["inputSchema"]);
    assert!(compatibility["description"]
        .as_str()
        .is_some_and(|description| description.contains("Compatibility alias")));

    let opened = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"watcher_open","arguments":{
            "assignmentId":"interrupt-test",
            "parent":{"id":"parent-task"},
            "watcher":{"id":"native-watcher"},
            "targets":[{"threadId":"target-thread"}],
            "ttlSeconds":60
        }}
    }))?;
    let opened = tool_payload(&opened)?;
    let session = opened["sessionId"].as_str().ok_or("session id")?.to_owned();
    let parent_token = opened["parentToken"].as_str().ok_or("parent token")?.to_owned();
    let watcher_token = opened["watcherToken"].as_str().ok_or("watcher token")?.to_owned();

    client.send_without_read(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"timeoutMs":30000
        }}
    }))?;
    let duplicate = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"watcher_health","arguments":{
            "sessionId":session,"token":parent_token
        }}
    }))?;
    assert_eq!(duplicate["error"]["code"], -32600);
    let mut waiting = false;
    for request_id in 40..140 {
        let health = client.send(&json!({
            "jsonrpc":"2.0","id":request_id,"method":"tools/call",
            "params":{"name":"watcher_health","arguments":{
                "sessionId":session,"token":parent_token
            }}
        }))?;
        waiting = tool_payload(&health)?["waiting"].as_bool().unwrap_or(false);
        if waiting {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(waiting, "waiter did not acquire wait.lock before cancellation");
    thread::sleep(Duration::from_millis(2));
    client.send_without_read(&json!({
        "jsonrpc":"2.0","method":"notifications/cancelled",
        "params":{"requestId":4,"reason":"user input"}
    }))?;
    let replacement = client.send(&json!({
        "jsonrpc":"2.0","id":41,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":0
        }}
    }))?;
    assert_eq!(tool_payload(&replacement)?["status"], "timeout");
    client.send_without_read(&json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"watcher_health","arguments":{
            "sessionId":session,"token":parent_token
        }}
    }))?;
    let response = client.read_frame()?;
    assert_eq!(response["id"], 5, "cancelled wait must not send a late response");
    assert_eq!(tool_payload(&response)?["waiting"], false);

    let reported = client.send(&json!({
        "jsonrpc":"2.0","id":6,"method":"tools/call",
        "params":{"name":"watcher_report","arguments":{
            "sessionId":session,"watcherToken":watcher_token,
            "target":{"threadId":"target-thread"},"eventId":"after-interrupt",
            "kind":"gate_ready","summary":"event after cancelled wait"
        }}
    }))?;
    assert_eq!(tool_payload(&reported)?["status"], "accepted");
    let duplicate_report = client.send(&json!({
        "jsonrpc":"2.0","id":61,"method":"tools/call",
        "params":{"name":"watcher_report","arguments":{
            "sessionId":session,"watcherToken":watcher_token,
            "target":{"threadId":"target-thread"},"eventId":"after-interrupt",
            "kind":"gate_ready","summary":"event after cancelled wait"
        }}
    }))?;
    assert_eq!(tool_payload(&duplicate_report)?["status"], "duplicate");
    let conflict = client.send(&json!({
        "jsonrpc":"2.0","id":62,"method":"tools/call",
        "params":{"name":"watcher_report","arguments":{
            "sessionId":session,"watcherToken":watcher_token,
            "target":{"threadId":"target-thread"},"eventId":"after-interrupt",
            "kind":"gate_ready","summary":"changed material event"
        }}
    }))?;
    assert_eq!(conflict["error"]["message"], "watcher eventId conflicts with an existing report");

    let waited = client.send(&json!({
        "jsonrpc":"2.0","id":7,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":1000
        }}
    }))?;
    let waited = tool_payload(&waited)?;
    assert_eq!(waited["status"], "event");
    assert_eq!(waited["events"].as_array().ok_or("events")?.len(), 1);
    let cursor = waited["nextCursor"].clone();
    assert!(cursor.as_str().is_some(), "watcher cursors must round-trip as strings");
    let empty = client.send(&json!({
        "jsonrpc":"2.0","id":71,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{
            "sessionId":session,"parentToken":parent_token,
            "cursor":cursor,"timeoutMs":0
        }}
    }))?;
    assert_eq!(tool_payload(&empty)?["status"], "timeout");

    let watcher_cancel = client.send(&json!({
        "jsonrpc":"2.0","id":8,"method":"tools/call",
        "params":{"name":"watcher_cancel","arguments":{
            "sessionId":session,"parentToken":watcher_token
        }}
    }))?;
    assert_eq!(watcher_cancel["error"]["code"], -32000);

    let cancelled = client.send(&json!({
        "jsonrpc":"2.0","id":9,"method":"tools/call",
        "params":{"name":"watcher_cancel","arguments":{
            "sessionId":session,"parentToken":parent_token
        }}
    }))?;
    assert_eq!(tool_payload(&cancelled)?["status"], "cancelled");
    Ok(())
}

#[test]
fn watcher_wait_and_legacy_alias_return_the_same_payload(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
    }))?;
    let opened = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"watcher_open","arguments":{
            "assignmentId":"alias-result-test",
            "parent":{"id":"parent-task"},
            "watcher":{"id":"native-watcher"},
            "targets":[{"threadId":"target-thread"}],
            "ttlSeconds":60
        }}
    }))?;
    let opened = tool_payload(&opened)?;
    let session = opened["sessionId"].as_str().ok_or("session id")?.to_owned();
    let parent_token = opened["parentToken"].as_str().ok_or("parent token")?.to_owned();
    let watcher_token = opened["watcherToken"].as_str().ok_or("watcher token")?.to_owned();
    let reported = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"watcher_report","arguments":{
            "sessionId":session,"watcherToken":watcher_token,
            "target":{"threadId":"target-thread"},"eventId":"same-payload",
            "kind":"gate_ready","summary":"same payload"
        }}
    }))?;
    assert_eq!(tool_payload(&reported)?["status"], "accepted");

    let arguments = json!({
        "sessionId":session,"parentToken":parent_token,
        "cursor":"0","maxReports":1,"timeoutMs":0
    });
    let canonical = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":arguments}
    }))?;
    let compatibility = client.send(&json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"wait_watcher","arguments":arguments}
    }))?;
    assert_eq!(tool_payload(&canonical)?, tool_payload(&compatibility)?);
    Ok(())
}
