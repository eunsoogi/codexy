use super::*;
use std::fs;
use std::sync::{Arc, Barrier};
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
    let preserved = unknown.join("preserve-me");
    fs::write(&preserved, b"unknown data")?;

    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    open_session(&mut client, "quarantine-recovery", 2)?;
    assert!(!quarantine.exists(), "owned quarantine was not recovered");
    assert!(preserved.is_file(), "unknown quarantine data was removed");
    Ok(())
}

#[test]
fn startup_resumes_recovery_when_a_lock_was_deleted_before_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let quarantine = state
        .path()
        .join(format!(".codexy-watcher-reclaim-{:032x}", 1));
    fs::create_dir(&quarantine)?;
    for name in ["wait.lock", "session.json", "health.json", "events.jsonl"] {
        fs::write(quarantine.join(name), b"owned")?;
    }

    let mut client = watcher_client(state.path())?;
    initialize(&mut client)?;
    open_session(&mut client, "partial-quarantine", 2)?;
    assert!(
        !quarantine.exists(),
        "recovery did not resume after state.lock was deleted"
    );
    Ok(())
}

#[test]
fn startup_scans_past_invalid_quarantines_without_starving_later_cleanup(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut invalid = Vec::new();
    for index in 0..80 {
        let quarantine = state
            .path()
            .join(format!(".codexy-watcher-reclaim-{index:032x}"));
        fs::create_dir(&quarantine)?;
        fs::write(quarantine.join("preserve-me"), b"unknown data")?;
        invalid.push(quarantine);
    }
    let valid = state
        .path()
        .join(format!(".codexy-watcher-reclaim-{:032x}", 80));
    fs::create_dir(&valid)?;
    for name in ["state.lock", "wait.lock", "session.json"] {
        fs::write(valid.join(name), b"owned")?;
    }

    for request_id in 2..=4 {
        let mut client = watcher_client(state.path())?;
        initialize(&mut client)?;
        open_session(
            &mut client,
            &format!("bounded-quarantine-scan-{request_id}"),
            request_id,
        )?;
        drop(client);
        if !valid.exists() {
            break;
        }
    }
    assert!(!valid.exists(), "later valid quarantine was starved");
    for quarantine in invalid {
        assert!(quarantine.join("preserve-me").is_file());
    }
    Ok(())
}

#[test]
fn concurrent_startup_recovery_uses_one_bounded_transition_lock(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = tempfile::tempdir()?;
    let mut quarantines = Vec::new();
    for index in 0..4 {
        let quarantine = state
            .path()
            .join(format!(".codexy-watcher-reclaim-{index:032x}"));
        fs::create_dir(&quarantine)?;
        for name in ["state.lock", "wait.lock", "session.json"] {
            fs::write(quarantine.join(name), b"owned")?;
        }
        quarantines.push(quarantine);
    }

    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for index in 0..2 {
        let barrier = Arc::clone(&barrier);
        let state = state.path().to_owned();
        workers.push(thread::spawn(move || -> Result<(), String> {
            barrier.wait();
            let mut client = watcher_client(&state).map_err(|error| error.to_string())?;
            initialize(&mut client).map_err(|error| error.to_string())?;
            open_session(&mut client, &format!("concurrent-recovery-{index}"), index + 2)
                .map_err(|error| error.to_string())?;
            Ok(())
        }));
    }
    for worker in workers {
        worker
            .join()
            .map_err(|_| "recovery worker panicked")??;
    }
    for quarantine in quarantines {
        assert!(!quarantine.exists(), "owned quarantine was not recovered");
    }
    let root = state.path().join("codexy-watcher");
    assert!(root.join(".reclaim.lock").is_file());
    assert!(!fs::read_dir(root)?.any(|entry| {
        entry
            .ok()
            .and_then(|entry| entry.file_name().into_string().ok())
            .is_some_and(|name| name.starts_with(".reclaim-"))
    }));
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
