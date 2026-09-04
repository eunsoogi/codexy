use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use crate::support::TestResult;

pub(super) const CHILD: &str = "01a03690-a037-7141-af1c-1d7cdb093087";
pub(super) const PARENT: &str = "01a02ebc-804e-7702-a0ca-503c50741db8";
pub(super) const OTHER: &str = "01a031f2-da35-7960-9686-502b7e373676";
pub(super) const ROUTING_FIELD: &str = "codexy_thread_delivery";

pub(super) fn route(
    direction: &str,
    sender: &str,
    recipient: &str,
    model: &str,
    thinking: &str,
) -> Value {
    json!({
        "schema":"codexy.thread-delivery.v2",
        "authenticated":true,
        "direction":direction,
        "sender_thread_id":sender,
        "target_thread_id":recipient,
        "target_model":model,
        "target_thinking":thinking
    })
}

pub(super) fn input(recipient: &str, model: &str, thinking: &str, prompt: &str) -> Value {
    json!({
        "threadId":recipient,
        "model":model,
        "thinking":thinking,
        "prompt":prompt
    })
}

pub(super) fn plugin_roots(temp: &Path) -> TestResult<[PathBuf; 2]> {
    let installed = temp.join("installed");
    crate::support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &installed,
    )?;
    Ok([
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        installed,
    ])
}

pub(super) fn run(
    root: &Path,
    event: &str,
    transcript: &Path,
    session: Option<&str>,
    tool_input: Value,
    routing: Option<Value>,
) -> TestResult<std::process::Output> {
    let mut payload = json!({
        "hook_event_name":event,
        "tool_name":"codex_app__send_message_to_thread",
        "tool_input":tool_input,
        "transcript_path":transcript
    });
    if let Some(session) = session {
        payload["session_id"] = json!(session);
    }
    if let Some(routing) = routing {
        payload[ROUTING_FIELD] = routing;
    }
    let payload = serde_json::to_vec(&payload)?;
    let mut command = launcher(root);
    let mut child = command
        .arg(event)
        .env("PLUGIN_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().ok_or("hook stdin")?.write_all(&payload)?;
    Ok(child.wait_with_output()?)
}

pub(super) fn reason(output: std::process::Output, event: &str) -> TestResult<String> {
    assert!(output.status.success(), "hook failed");
    assert!(output.stderr.is_empty(), "hook wrote stderr");
    let denial: Value = serde_json::from_slice(&output.stdout)?;
    let specific = &denial["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], event);
    let reason = if event == "PermissionRequest" {
        specific["decision"]["message"].as_str().ok_or("permission message")?
    } else {
        specific["permissionDecisionReason"].as_str().ok_or("permission reason")?
    };
    Ok(reason.to_owned())
}

pub(super) fn assert_admitted(
    output: std::process::Output,
    event: &str,
    label: &str,
) -> TestResult {
    assert!(output.status.success(), "{event} {label}: hook failed");
    assert!(
        output.stdout.is_empty(),
        "{event} {label}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty(), "{event} {label}: hook wrote stderr");
    Ok(())
}

pub(super) fn assert_denied(
    output: std::process::Output,
    event: &str,
    expected: &str,
) -> TestResult {
    let reason = reason(output, event)?;
    assert!(reason.contains(expected), "{event}: {reason}");
    Ok(())
}

#[cfg(windows)]
fn launcher(root: &Path) -> Command {
    let mut command = Command::new("cmd");
    command
        .arg("/d")
        .arg("/c")
        .arg(root.join("hooks/codexy-thread-delivery.cmd"));
    command
}

#[cfg(not(windows))]
fn launcher(root: &Path) -> Command {
    Command::new(root.join("hooks/codexy-thread-delivery.sh"))
}
