use super::*;
use std::thread;
use std::time::{Duration, Instant};

// A finite bound keeps failed cancellation cleanup from holding the test process open.
const TEST_WAIT_MS: u64 = 30_000;

#[path = "user_prompt_submit/session_binding_open.rs"]
mod session_binding_open;

// A prompt releases a wait only when one live request binding matches its host
// session; ambiguous or different-session inputs must leave the waits running.
#[test]
fn user_prompt_submit_cancels_only_a_unique_wait_for_its_host_session() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let plugin = root.join("plugins/codexy");
    let temp = tempfile::tempdir()?;
    let cache = temp.path().join("runtime-cache");
    let state = temp.path().join("state");
    let source = Path::new(env!("CARGO_BIN_EXE_codexy-mcp-watcher"));
    install_cached(&plugin, &cache, "linux-x86_64", source)?;

    let mut setup = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut setup)?;
    let (first_session, first_parent, _) = watcher_state::open_session(&mut setup, "first", 2)?;
    drop(setup);

    let (mut first, first_binding) = begin_wait(
        &plugin,
        &cache,
        &state,
        &first_session,
        &first_parent,
        "turn-1",
        "tool-1",
    )?;
    let (second_session, second_parent, second_watcher) =
        session_binding_open::open_after_live_binding(
            &plugin,
            &cache,
            &state,
            &mut first,
            &first_binding,
            &first_session,
            &first_parent,
        )?;

    let other_session = run_hook(
        &plugin,
        &cache,
        &state,
        Path::new(""),
        "linux-x86_64",
        "UserPromptSubmit",
        json!({"hook_event_name":"UserPromptSubmit","session_id":"other-session",
            "turn_id":"turn-3","prompt":"next input"}),
    )?;
    assert!(other_session.status.success() && other_session.stdout.is_empty());
    assert!(!binding_cancelled(&state, &first_binding));
    assert!(wait_is_active(&state, &first_session, &first_parent)?);

    let (mut second, second_binding) = begin_wait(
        &plugin,
        &cache,
        &state,
        &second_session,
        &second_parent,
        "turn-2",
        "tool-2",
    )?;

    let ambiguous = run_hook(
        &plugin,
        &cache,
        &state,
        Path::new(""),
        "linux-x86_64",
        "UserPromptSubmit",
        json!({"hook_event_name":"UserPromptSubmit","session_id":"main-session",
            "turn_id":"turn-3","prompt":"next input"}),
    )?;
    assert!(ambiguous.status.success() && ambiguous.stdout.is_empty());
    assert!(!binding_cancelled(&state, &first_binding));
    assert!(!binding_cancelled(&state, &second_binding));
    assert!(wait_is_active(&state, &first_session, &first_parent)?);
    assert!(wait_is_active(&state, &second_session, &second_parent)?);

    let interrupted = run_hook(
        &plugin,
        &cache,
        &state,
        Path::new(""),
        "linux-x86_64",
        "Interrupt",
        json!({"hook_event_name":"Interrupt","session_id":"main-session","turn_id":"turn-1"}),
    )?;
    assert!(interrupted.status.success() && interrupted.stdout.is_empty());
    assert_eq!(
        watcher_state::tool_payload(&first.read_frame()?)?["status"],
        "cancelled"
    );

    let started = Instant::now();
    let submitted = run_hook(
        &plugin,
        &cache,
        &state,
        Path::new(""),
        "linux-x86_64",
        "UserPromptSubmit",
        json!({"hook_event_name":"UserPromptSubmit","session_id":"main-session",
            "turn_id":"turn-3","prompt":"continue with the next report"}),
    )?;
    assert!(submitted.status.success() && submitted.stdout.is_empty());
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let response = second.read_frame().map_err(|error| error.to_string());
        let _ = sender.send(response);
    });
    let response = receiver.recv_timeout(Duration::from_secs(2).saturating_sub(started.elapsed()));
    let response = match response {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => return Err(error.into()),
        Err(_) => {
            let cleanup = run_hook(
                &plugin,
                &cache,
                &state,
                Path::new(""),
                "linux-x86_64",
                "Interrupt",
                json!({"hook_event_name":"Interrupt","session_id":"main-session","turn_id":"turn-2"}),
            )?;
            assert!(cleanup.status.success());
            let _ = receiver.recv_timeout(Duration::from_secs(2));
            return Err("UserPromptSubmit did not release the pending wait promptly".into());
        }
    };
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(
        watcher_state::tool_payload(&response)?["status"],
        "cancelled"
    );

    let mut observer = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut observer)?;
    let health = observer.send(&json!({"jsonrpc":"2.0","id":30,"method":"tools/call",
        "params":{"name":"watcher_health","arguments":{"sessionId":second_session,
        "token":second_parent}}}))?;
    let health = watcher_state::tool_payload(&health)?;
    assert_eq!(health["status"], "active");
    assert_eq!(health["waiting"], false);

    let report = observer.send(&json!({"jsonrpc":"2.0","id":31,"method":"tools/call",
        "params":{"name":"watcher_report","arguments":{"sessionId":second_session,
        "watcherToken":second_watcher,"target":{"threadId":"target"},
        "eventId":"after-prompt","kind":"gate_ready","summary":"event after prompt"}}}))?;
    assert_eq!(watcher_state::tool_payload(&report)?["status"], "accepted");
    let mut resumed = watcher_state::watcher_client(&state)?;
    watcher_state::initialize(&mut resumed)?;
    let result = resumed.send(&json!({"jsonrpc":"2.0","id":32,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{"sessionId":second_session,
        "parentToken":second_parent,"cursor":0,"timeoutMs":0}}}))?;
    assert_eq!(watcher_state::tool_payload(&result)?["status"], "event");
    Ok(())
}

fn begin_wait(
    plugin: &Path,
    cache: &Path,
    state: &Path,
    watcher_session: &str,
    parent_token: &str,
    turn_id: &str,
    tool_use_id: &str,
) -> Result<(McpClient, String), Box<dyn std::error::Error>> {
    let output = run_hook(
        plugin,
        cache,
        state,
        Path::new(""),
        "linux-x86_64",
        "PreToolUse",
        json!({"hook_event_name":"PreToolUse","tool_name":"mcp__codexy-watcher__watcher_wait",
            "session_id":"main-session","turn_id":turn_id,"tool_use_id":tool_use_id,
            "tool_input":{"sessionId":watcher_session,"parentToken":parent_token,
            "timeoutMs":TEST_WAIT_MS}}),
    )?;
    let binding: Value = serde_json::from_slice(&output.stdout)?;
    let binding = binding["hookSpecificOutput"]["updatedInput"]["requestBinding"]
        .as_str()
        .ok_or("request binding")?
        .to_owned();
    let mut reader = watcher_state::watcher_client(state)?;
    watcher_state::initialize(&mut reader)?;
    reader.send_without_read(&json!({"jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"watcher_wait","arguments":{"sessionId":watcher_session,
        "parentToken":parent_token,"requestBinding":binding,"timeoutMs":TEST_WAIT_MS}}}))?;

    let mut observer = watcher_state::watcher_client(state)?;
    watcher_state::initialize(&mut observer)?;
    for request_id in 10..110 {
        let health = observer.send(
            &json!({"jsonrpc":"2.0","id":request_id,"method":"tools/call",
            "params":{"name":"watcher_health","arguments":{"sessionId":watcher_session,
            "token":parent_token}}}),
        )?;
        if watcher_state::tool_payload(&health)?["waiting"] == true {
            return Ok((reader, binding));
        }
        thread::sleep(Duration::from_millis(1));
    }
    Err("bound watcher_wait did not become active".into())
}

fn wait_is_active(
    state: &Path,
    session: &str,
    parent_token: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut observer = watcher_state::watcher_client(state)?;
    watcher_state::initialize(&mut observer)?;
    let health = observer.send(&json!({"jsonrpc":"2.0","id":40,"method":"tools/call",
        "params":{"name":"watcher_health","arguments":{"sessionId":session,
        "token":parent_token}}}))?;
    Ok(watcher_state::tool_payload(&health)?["waiting"] == true)
}
