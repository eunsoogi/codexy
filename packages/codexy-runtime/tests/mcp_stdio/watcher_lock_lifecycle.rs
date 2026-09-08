use super::*;
use std::fs;
use std::thread;
use std::time::Duration;

use super::watcher_state::{initialize, open_session, tool_payload, watcher_client};

#[test]
fn wait_lock_is_released_when_its_owner_process_dies() -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut setup = watcher_client(state.path())?;
    initialize(&mut setup)?;
    let (session, parent_token, _) = open_session(&mut setup, "process-death-lock", 2)?;
    drop(setup);

    let mut observer = watcher_client(state.path())?;
    initialize(&mut observer)?;
    let idle_health = observer.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"watcher_health","arguments":{
            "sessionId":session,"token":parent_token
        }}
    }))?;
    assert_eq!(tool_payload(&idle_health)?["waiting"], false);

    let mut holder = watcher_client(state.path())?;
    initialize(&mut holder)?;
    holder.send_without_read(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"wait_watcher","arguments":{
            "sessionId":session,"parentToken":parent_token,"timeoutMs":30000
        }}
    }))?;

    let mut waiting = false;
    for request_id in 10..110 {
        let health = observer.send(&json!({
            "jsonrpc":"2.0","id":request_id,"method":"tools/call",
            "params":{"name":"watcher_health","arguments":{
                "sessionId":session,"token":parent_token
            }}
        }))?;
        if health.get("error").is_none() && tool_payload(&health)?["waiting"] == true {
            waiting = true;
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(waiting, "waiter did not acquire wait.lock before termination");
    drop(observer);

    holder.child.kill()?;
    holder.child.wait()?;
    drop(holder);

    let mut replacement = watcher_client(state.path())?;
    initialize(&mut replacement)?;
    let response = replacement.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"wait_watcher","arguments":{
            "sessionId":session,"parentToken":parent_token,"cursor":0,"timeoutMs":0
        }}
    }))?;
    assert_eq!(tool_payload(&response)?["status"], "timeout");
    let released_health = replacement.send(&json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"watcher_health","arguments":{
            "sessionId":session,"token":parent_token
        }}
    }))?;
    assert_eq!(tool_payload(&released_health)?["waiting"], false);
    Ok(())
}

#[test]
fn startup_recovers_owned_quarantine_without_deleting_unknown_data(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let quarantine = state
        .path()
        .join(format!(".codexy-watcher-reclaim-{}", "0".repeat(32)));
    fs::create_dir(&quarantine)?;
    for name in ["state.lock", "wait.lock", "session.json"] {
        fs::write(quarantine.join(name), b"owned")?;
    }
    let unknown = state.path().join(".codexy-watcher-reclaim-not-owned");
    fs::create_dir(&unknown)?;
    let sentinel = unknown.join("preserve-me");
    fs::write(&sentinel, b"unknown data")?;

    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    open_session(&mut client, "quarantine-recovery", 2)?;
    assert!(!quarantine.exists(), "owned quarantine was not recovered");
    assert!(sentinel.is_file(), "unknown quarantine data was removed");
    Ok(())
}

#[test]
fn late_access_does_not_recreate_a_reclaimed_session_directory(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    let opened = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"watcher_open","arguments":{
            "assignmentId":"short-lived-session","parent":{"id":"parent"},
            "watcher":{"id":"watcher"},"targets":[{"threadId":"target"}],
            "ttlSeconds":1
        }}
    }))?;
    let opened = tool_payload(&opened)?;
    let session = opened["sessionId"].as_str().ok_or("missing session")?.to_owned();
    let parent_token = opened["parentToken"].as_str().ok_or("missing token")?.to_owned();
    let path = state.path().join("codexy-watcher").join(&session);

    thread::sleep(Duration::from_millis(1_200));
    open_session(&mut client, "reclaim-trigger", 3)?;
    assert!(!path.exists(), "expired session directory was not reclaimed");

    let late_cancel = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"watcher_cancel","arguments":{
            "sessionId":session,"parentToken":parent_token
        }}
    }))?;
    assert!(late_cancel.get("error").is_some(), "late access unexpectedly succeeded");
    assert!(!path.exists(), "late access recreated the reclaimed directory");
    Ok(())
}
