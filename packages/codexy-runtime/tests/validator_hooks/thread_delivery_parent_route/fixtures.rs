use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::support::TestResult;

pub(super) const CHILD: &str = "01a03690-a037-7141-af1c-1d7cdb093087";
pub(super) const PARENT: &str = "01a02ebc-804e-7702-a0ca-503c50741db8";
pub(super) const OTHER: &str = "01a031f2-da35-7960-9686-502b7e373676";
pub(super) const TOOLS: &[&str] = &[
    "codex_app__send_message_to_thread",
    "mcp__codex_app__send_message_to_thread",
];

pub(super) fn child_transcript(session: &str, parents: &[&str]) -> Vec<u8> {
    let content = parents
        .iter()
        .map(|parent| json!({"type":"input_text","text":delegation(parent, "Implement the owned lane.")}))
        .collect::<Vec<_>>();
    transcript(session, content)
}

pub(super) fn child_transcript_text(session: &str, text: &str) -> Vec<u8> {
    transcript(session, vec![json!({"type":"input_text","text":text})])
}

fn transcript(session: &str, content: Vec<Value>) -> Vec<u8> {
    let lines = [
        json!({"type":"session_meta","payload":{"id":session,"session_id":session}}).to_string(),
        json!({"type":"turn_context","payload":{}}).to_string(),
        json!({"type":"response_item","payload":{"type":"message","role":"user","content":content}}).to_string(),
    ];
    format!("{}\n", lines.join("\n")).into_bytes()
}

pub(super) fn root_transcript(session: &str) -> Vec<u8> {
    transcript(
        session,
        vec![json!({"type":"input_text","text":"Own the release orchestration."})],
    )
}

pub(super) fn completed_delivery(
    recipient: &str,
    model: &str,
    thinking: &str,
    prompt: &str,
) -> Vec<u8> {
    format!(
        "{}\n",
        json!({
            "type":"event_msg",
            "payload":{
                "type":"item_completed",
                "item":{
                    "type":"McpToolCall",
                    "server":"codex_app",
                    "tool":"send_message_to_thread",
                    "status":"completed",
                    "arguments":{
                        "threadId":recipient,
                        "model":model,
                        "thinking":thinking,
                        "prompt":prompt
                    }
                }
            }
        })
    )
    .into_bytes()
}

pub(super) fn delegation(parent: &str, input: &str) -> String {
    format!("<codex_delegation>\n<source_thread_id>{parent}</source_thread_id>\n<input>{input}</input>\n</codex_delegation>")
}

pub(super) fn run(
    transcript: &Path,
    model: &str,
    thinking: &str,
) -> TestResult<std::process::Output> {
    run_to(transcript, CHILD, PARENT, model, thinking)
}

pub(super) fn run_to(
    transcript: &Path,
    session: &str,
    recipient: &str,
    model: &str,
    thinking: &str,
) -> TestResult<std::process::Output> {
    run_route_at(
        &codexy_runtime::paths::repository_root().join("plugins/codexy"),
        transcript,
        session,
        recipient,
        model,
        thinking,
        "codex_app__send_message_to_thread",
        "PreToolUse",
    )
}

pub(super) fn run_route_prompt(
    transcript: &Path,
    session: &str,
    recipient: &str,
    model: &str,
    thinking: &str,
    prompt: &str,
) -> TestResult<std::process::Output> {
    run_payload(json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"codex_app__send_message_to_thread",
        "session_id":session,
        "transcript_path":transcript,
        "tool_input":{
            "threadId":recipient,
            "model":model,
            "thinking":thinking,
            "prompt":prompt
        }
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_route_at(
    root: &Path,
    transcript: &Path,
    session: &str,
    recipient: &str,
    model: &str,
    thinking: &str,
    tool: &str,
    event: &str,
) -> TestResult<std::process::Output> {
    run_payload_at(root, json!({
        "hook_event_name":event,
        "tool_name":tool,
        "session_id":session,
        "transcript_path":transcript,
        "tool_input":{"threadId":recipient,"model":model,"thinking":thinking,"prompt":"Continue the owned lane."}
    }))
}

pub(super) fn run_payload(payload: Value) -> TestResult<std::process::Output> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    run_payload_at(&root, payload)
}

pub(super) fn run_payload_at(
    root: &Path,
    payload: Value,
) -> TestResult<std::process::Output> {
    let event = payload["hook_event_name"]
        .as_str()
        .ok_or("hook event")?
        .to_owned();
    let payload = serde_json::to_vec(&payload)?;
    let mut command = launcher(root);
    let mut child = command
        .arg(event)
        .env("PLUGIN_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(&payload)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait()?.is_none() {
        if Instant::now() >= deadline {
            child.kill()?;
            return Err("hook runtime timed out".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(child.wait_with_output()?)
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

pub(super) fn assert_denied(output: std::process::Output) -> TestResult {
    assert_denied_with(output, "EXPECTED_RECIPIENT")
}

pub(super) fn assert_denied_with(
    output: std::process::Output,
    diagnostic: &str,
) -> TestResult {
    assert!(output.status.success(), "hook runtime failed");
    let denial: Value = serde_json::from_slice(&output.stdout)?;
    let reason = denial["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .or_else(|| denial["hookSpecificOutput"]["decision"]["message"].as_str())
        .ok_or("denial reason")?;
    assert!(reason.contains(diagnostic), "unexpected denial: {reason}");
    Ok(())
}

pub(super) fn assert_admitted(output: std::process::Output) -> TestResult {
    assert!(output.status.success(), "hook runtime failed");
    assert!(
        output.stdout.is_empty(),
        "recipient-bound route was denied: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}
