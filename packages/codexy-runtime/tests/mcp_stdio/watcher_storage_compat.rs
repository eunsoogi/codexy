//! Replays stored Watcher events written before public response projection.

use super::*;

#[test]
fn pre_change_stored_event_replays_without_migration_or_mutation() -> Result<(), String> {
    let state = tempfile::tempdir().map_err(|error| error.to_string())?;
    let mut setup = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut setup).map_err(|error| error.to_string())?;
    let (session, parent_token, _) =
        open_session(&mut setup, "pre-change-event-replay", 2).map_err(|error| error.to_string())?;

    // This literal event uses the stored shape from before the wait projection existed.
    let stored_event = json!({
        "eventId":"evt-pre-change-fixed",
        "sequence":1,
        "kind":"gate_ready",
        "target":{"threadId":"target"},
        "summary":"기존 기록 그대로 🚀",
        "observedAtMs":1_700_000_000_123_u64,
        "evidence":["commit:legacy", "log:한글-완료"],
        "fingerprint":"a".repeat(64)
    });
    let path = event_path(state.path(), &session);
    let mut legacy_line = serde_json::to_vec(&stored_event).map_err(|error| error.to_string())?;
    legacy_line.push(b'\n');
    std::fs::write(&path, legacy_line).map_err(|error| error.to_string())?;
    let session_path = state
        .path()
        .join("codexy-watcher")
        .join(&session)
        .join("session.json");
    let mut stored_session: Value =
        serde_json::from_slice(&std::fs::read(&session_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    stored_session["nextSequence"] = json!(1);
    std::fs::write(
        &session_path,
        serde_json::to_vec(&stored_session).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let expected = json!({
        "eventId":"evt-pre-change-fixed",
        "sequence":1,
        "kind":"gate_ready",
        "target":{"threadId":"target"},
        "summary":"기존 기록 그대로 🚀",
        "observedAtMs":1_700_000_000_123_u64,
        "evidence":["commit:legacy", "log:한글-완료"]
    });
    let before = std::fs::read(&path).map_err(|error| error.to_string())?;
    drop(setup);

    let mut reader = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut reader).map_err(|error| error.to_string())?;
    let first = wait_all(&mut reader, &session, &parent_token, 3)?;
    if first["status"] != "event" || first["nextCursor"] != "1" {
        return Err(format!("pre-change event did not replay: {first}"));
    }
    let first_events = first["events"]
        .as_array()
        .ok_or_else(|| format!("pre-change wait omitted events: {first}"))?;
    if first_events.len() != 1 || first_events[0] != expected {
        return Err(format!("pre-change event projection changed: {first}"));
    }
    if std::fs::read(&path).map_err(|error| error.to_string())? != before {
        return Err("read-only wait rewrote the pre-change event record".to_owned());
    }
    drop(reader);

    let mut restarted = watcher_client(state.path()).map_err(|error| error.to_string())?;
    initialize(&mut restarted).map_err(|error| error.to_string())?;
    let replayed = wait_all(&mut restarted, &session, &parent_token, 4)?;
    if replayed["events"][0] != expected || replayed["nextCursor"] != "1" {
        return Err(format!("restart changed the pre-change event identity: {replayed}"));
    }
    if std::fs::read(&path).map_err(|error| error.to_string())? != before {
        return Err("restart replay rewrote the pre-change event record".to_owned());
    }
    Ok(())
}
