use super::*;

#[path = "schema_contract/report_modes.rs"]
mod report_modes;
#[path = "schema_contract/report_text.rs"]
mod report_text;
#[path = "schema_contract/tools_list.rs"]
mod tools_list;

pub(super) fn watcher_session()
-> Result<(tempfile::TempDir, McpClient, String, String, Value), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut client = super::watcher_client(state.path())?;
    client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
    }))?;
    let (session, watcher_token, target) = open_session(&mut client)?;
    Ok((state, client, session, watcher_token, target))
}

fn open_session(
    client: &mut McpClient,
) -> Result<(String, String, Value), Box<dyn std::error::Error>> {
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"watcher_open","arguments":{
            "assignmentId":"schema-contract",
            "parent":{"id":"parent-task"},
            "watcher":{"id":"native-watcher"},
            "targets":[{"threadId":"target-thread"}],
            "ttlSeconds":60
        }}
    }))?;
    let opened = super::tool_payload(&response)?;
    Ok((
        opened["sessionId"].as_str().ok_or("session id")?.to_owned(),
        opened["watcherToken"]
            .as_str()
            .ok_or("watcher token")?
            .to_owned(),
        json!({"threadId":"target-thread"}),
    ))
}

pub(super) fn call_report(
    client: &mut McpClient,
    request_id: u64,
    session: &str,
    watcher_token: &str,
    fields: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut arguments = json!({
        "sessionId": session,
        "watcherToken": watcher_token,
    })
    .as_object()
    .cloned()
    .ok_or("report arguments must be an object")?;
    let fields = fields
        .as_object()
        .ok_or("test report fields must be an object")?;
    arguments.extend(fields.clone());
    client.send(&json!({
        "jsonrpc":"2.0",
        "id":request_id,
        "method":"tools/call",
        "params":{"name":"watcher_report","arguments":arguments}
    }))
}

pub(super) fn assert_status(
    response: &Value,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(super::tool_payload(response)?["status"], expected);
    Ok(())
}

pub(super) fn assert_error_contains(response: &Value, expected: &str) {
    assert!(
        response["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains(expected)),
        "expected error message containing {expected:?}, got {response}"
    );
}
