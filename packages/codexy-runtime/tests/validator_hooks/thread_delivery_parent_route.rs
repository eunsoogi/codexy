use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::support::TestResult;

const CHILD: &str = "01a03690-a037-7141-af1c-1d7cdb093087";
const PARENT: &str = "01a02ebc-804e-7702-a0ca-503c50741db8";
const OTHER: &str = "01a031f2-da35-7960-9686-502b7e373676";

#[test]
fn authoritative_and_installed_hooks_reject_missing_identity() -> TestResult {
    let payload = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"codex_app__send_message_to_thread",
        "tool_input":{"threadId":PARENT,"model":"gpt-5.6-luna","thinking":"max"}
    });
    assert_denied(run_payload(payload.clone())?)?;

    let temp = tempfile::tempdir()?;
    let installed = temp.path().join("installed");
    crate::support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &installed,
    )?;
    assert_denied(run_payload_at(&installed, payload)?)?;
    let malformed = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"codex_app__send_message_to_thread",
        "tool_input":{"threadId":PARENT,"thinking":""}
    });
    assert_denied(run_payload(malformed.clone())?)?;
    assert_denied(run_payload_at(&installed, malformed)?)?;
    Ok(())
}

#[test]
fn installed_hook_preserves_authenticated_parent_route() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("child.jsonl");
    std::fs::write(&transcript, child_transcript(CHILD, &[PARENT]))?;

    let wrong = run(&transcript, "gpt-5.6-luna", "max")?;
    assert!(wrong.status.success(), "hook runtime failed");
    assert!(!wrong.stdout.is_empty(), "wrong parent route was admitted");
    let denial: Value = serde_json::from_slice(&wrong.stdout)?;
    assert!(
        denial["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("EXPECTED_RECIPIENT")),
        "wrong parent route was admitted: {}",
        String::from_utf8_lossy(&wrong.stdout)
    );

    let correct = run(&transcript, "gpt-5.6-sol", "medium")?;
    assert!(correct.status.success(), "hook runtime failed");
    assert!(correct.stdout.is_empty(), "correct parent route was denied");
    Ok(())
}

#[test]
fn installed_hook_fails_closed_for_untrusted_child_context() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("child.jsonl");
    for bytes in [
        child_transcript(OTHER, &[PARENT]),
        child_transcript(CHILD, &[PARENT, OTHER]),
        child_transcript_text(
            CHILD,
            &format!(
                "{}{}",
                delegation(PARENT, "First."),
                delegation(OTHER, "Second.")
            ),
        ),
        child_transcript(CHILD, &[OTHER]),
        b"not-json\n".to_vec(),
    ] {
        std::fs::write(&transcript, bytes)?;
        assert_denied(run(&transcript, "gpt-5.6-sol", "medium")?)?;
    }
    std::fs::File::create(&transcript)?.set_len(32 * 1024 * 1024 + 1)?;
    assert_denied(run(&transcript, "gpt-5.6-sol", "medium")?)?;
    assert_denied(run(temp.path(), "gpt-5.6-sol", "medium")?)?;
    #[cfg(unix)]
    {
        let link = temp.path().join("link.jsonl");
        std::os::unix::fs::symlink(&transcript, &link)?;
        assert_denied(run(&link, "gpt-5.6-sol", "medium")?)?;
        let fifo = temp.path().join("fifo.jsonl");
        assert!(Command::new("mkfifo").arg(&fifo).status()?.success());
        assert_denied(run(&fifo, "gpt-5.6-sol", "medium")?)?;
    }
    #[cfg(windows)]
    {
        let link = temp.path().join("link.jsonl");
        std::os::windows::fs::symlink_file(&transcript, &link)?;
        assert_denied(run(&link, "gpt-5.6-sol", "medium")?)?;
    }
    Ok(())
}

#[test]
fn installed_hook_preserves_root_to_child_and_rejects_partial_context() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("root.jsonl");
    std::fs::write(
        &transcript,
        format!(
            "{}\n{}\n{}\n{}\n",
            json!({"type":"session_meta","payload":{"id":PARENT,"session_id":PARENT}}),
            json!({"type":"turn_context","payload":{}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Document the literal <codex_delegation> marker in a root-owned task."}]}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":format!("<codex_delegation><source_thread_id>{PARENT}</source_thread_id><input>Later receipt.</input></codex_delegation>")}]}}),
        ),
    )?;
    let generic = run_to(&transcript, PARENT, CHILD, "gpt-5.6-luna", "max")?;
    assert!(generic.stdout.is_empty(), "root-to-child delivery was denied");
    let partial = run_payload(json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"codex_app__send_message_to_thread",
        "session_id":PARENT,
        "tool_input":{"threadId":PARENT,"model":"gpt-5.6-sol","thinking":"medium"}
    }))?;
    assert_denied(partial)?;
    Ok(())
}

fn child_transcript(session: &str, parents: &[&str]) -> Vec<u8> {
    let content = parents
        .iter()
        .map(|parent| json!({"type":"input_text","text":delegation(parent, "Implement the owned lane.")}))
        .collect::<Vec<_>>();
    transcript(session, content)
}

fn child_transcript_text(session: &str, text: &str) -> Vec<u8> {
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

fn delegation(parent: &str, input: &str) -> String {
    format!("<codex_delegation>\n<source_thread_id>{parent}</source_thread_id>\n<input>{input}</input>\n</codex_delegation>")
}

fn run(transcript: &Path, model: &str, thinking: &str) -> TestResult<std::process::Output> {
    run_to(transcript, CHILD, PARENT, model, thinking)
}

fn run_to(
    transcript: &Path,
    session: &str,
    recipient: &str,
    model: &str,
    thinking: &str,
) -> TestResult<std::process::Output> {
    run_payload(json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"codex_app__send_message_to_thread",
        "session_id":session,
        "transcript_path":transcript,
        "tool_input":{"threadId":recipient,"model":model,"thinking":thinking,"prompt":"receipt"}
    }))
}

fn run_payload(payload: Value) -> TestResult<std::process::Output> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    run_payload_at(&root, payload)
}

fn run_payload_at(root: &Path, payload: Value) -> TestResult<std::process::Output> {
    let payload = serde_json::to_vec(&payload)?;
    let mut command = launcher(&root);
    let mut child = command
        .arg("PreToolUse")
        .env("PLUGIN_ROOT", &root)
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

fn assert_denied(output: std::process::Output) -> TestResult {
    assert!(output.status.success(), "hook runtime failed");
    let denial: Value = serde_json::from_slice(&output.stdout)?;
    assert!(
        denial["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("EXPECTED_RECIPIENT")),
        "untrusted child context was admitted"
    );
    Ok(())
}
