use std::process::Command;

use serde_json::{Value, json};

use super::fixtures::{
    CHILD as WORKER_ID, OTHER, PARENT as ORCHESTRATOR_ID, assert_admitted, assert_denied,
    assert_denied_with, child_transcript, child_transcript_text, delegation, root_transcript, run,
    run_payload, run_payload_at, run_to,
};
use crate::support::TestResult;

#[test]
fn authoritative_and_installed_hooks_reject_missing_identity() -> TestResult {
    let payload = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"codex_app__send_message_to_thread",
        "tool_input":{"threadId":ORCHESTRATOR_ID,"model":"gpt-5.6-luna","thinking":"max"}
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
        "tool_input":{"threadId":ORCHESTRATOR_ID,"thinking":""}
    });
    assert_denied(run_payload(malformed.clone())?)?;
    assert_denied(run_payload_at(&installed, malformed)?)?;
    Ok(())
}

#[test]
fn installed_hook_preserves_authenticated_orchestrator_route() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("child.jsonl");
    std::fs::write(&transcript, child_transcript(WORKER_ID, &[ORCHESTRATOR_ID]))?;

    let wrong = run(&transcript, "gpt-5.6-luna", "max")?;
    assert!(wrong.status.success(), "hook runtime failed");
    assert!(!wrong.stdout.is_empty(), "wrong Orchestrator route was admitted");
    let denial: Value = serde_json::from_slice(&wrong.stdout)?;
    assert!(
        denial["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("UNSUPPORTED_MODEL")),
        "wrong Orchestrator route was admitted: {}",
        String::from_utf8_lossy(&wrong.stdout)
    );

    let correct = run(&transcript, "gpt-5.6-sol", "medium")?;
    assert!(correct.status.success(), "hook runtime failed");
    assert!(correct.stdout.is_empty(), "correct Orchestrator route was denied");
    Ok(())
}

#[test]
fn authentic_agent_created_thread_binds_orchestrator_from_create_thread_provenance() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("authentic-child.jsonl");
    std::fs::write(&transcript, child_transcript(WORKER_ID, &[ORCHESTRATOR_ID]))?;

    let wrong = run(&transcript, "gpt-5.6-luna", "max")?;
    assert_denied_with(wrong, "UNSUPPORTED_MODEL")?;
    assert_admitted(run(&transcript, "gpt-5.6-sol", "medium")?)?;
    Ok(())
}

#[test]
fn installed_hook_fails_closed_for_untrusted_worker_context() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("child.jsonl");
    for (bytes, expected) in [
        (child_transcript(OTHER, &[ORCHESTRATOR_ID]), "UNTRUSTED_CONTEXT"),
        (
            child_transcript(WORKER_ID, &[ORCHESTRATOR_ID, OTHER]),
            "UNTRUSTED_CONTEXT",
        ),
        (
            child_transcript_text(
                WORKER_ID,
                &format!(
                    "{}{}",
                    delegation(ORCHESTRATOR_ID, "First."),
                    delegation(OTHER, "Second.")
                ),
            ),
            "UNTRUSTED_CONTEXT",
        ),
        (child_transcript(WORKER_ID, &[OTHER]), "WRONG_RECIPIENT"),
        (b"not-json\n".to_vec(), "UNTRUSTED_CONTEXT"),
    ] {
        std::fs::write(&transcript, bytes)?;
        assert_denied_with(run(&transcript, "gpt-5.6-sol", "medium")?, expected)?;
    }
    std::fs::File::create(&transcript)?.set_len(32 * 1024 * 1024 + 1)?;
    assert_denied_with(run(&transcript, "gpt-5.6-sol", "medium")?, "UNTRUSTED_CONTEXT")?;
    assert_denied_with(run(temp.path(), "gpt-5.6-sol", "medium")?, "UNTRUSTED_CONTEXT")?;
    #[cfg(unix)]
    {
        let link = temp.path().join("link.jsonl");
        std::os::unix::fs::symlink(&transcript, &link)?;
        assert_denied_with(run(&link, "gpt-5.6-sol", "medium")?, "UNTRUSTED_CONTEXT")?;
        let fifo = temp.path().join("fifo.jsonl");
        assert!(Command::new("mkfifo").arg(&fifo).status()?.success());
        assert_denied_with(run(&fifo, "gpt-5.6-sol", "medium")?, "UNTRUSTED_CONTEXT")?;
    }
    #[cfg(windows)]
    {
        let link = temp.path().join("link.jsonl");
        std::os::windows::fs::symlink_file(&transcript, &link)?;
        assert_denied_with(run(&link, "gpt-5.6-sol", "medium")?, "UNTRUSTED_CONTEXT")?;
    }
    Ok(())
}

#[test]
fn installed_hook_binds_orchestrator_to_worker_delivery_to_worker_recipient_settings() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("root.jsonl");
    std::fs::write(
        &transcript,
        format!(
            "{}\n{}\n{}\n{}\n{}\n",
            json!({"type":"session_meta","payload":{"id":ORCHESTRATOR_ID,"session_id":ORCHESTRATOR_ID}}),
            json!({"type":"turn_context","payload":{}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Document the literal <codex_delegation> marker in an Orchestrator-owned task."}]}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":format!("<codex_delegation><source_thread_id>{ORCHESTRATOR_ID}</source_thread_id><input>Later receipt.</input></codex_delegation>")}]}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","name":"create_thread","namespace":"codex_app","output":json!({"threadId":WORKER_ID,"parentThreadId":ORCHESTRATOR_ID}).to_string()}}),
        ),
    )?;
    let worker_delivery = run_to(
        &transcript,
        ORCHESTRATOR_ID,
        WORKER_ID,
        "gpt-5.6-luna",
        "max",
    )?;
    assert!(
        worker_delivery.stdout.is_empty(),
        "Orchestrator-to-Worker delivery was denied"
    );
    let copied_sender = run_to(
        &transcript,
        ORCHESTRATOR_ID,
        WORKER_ID,
        "gpt-5.6-sol",
        "medium",
    )?;
    assert!(copied_sender.status.success(), "hook runtime failed");
    let denial: Value = serde_json::from_slice(&copied_sender.stdout)?;
    assert!(
        denial["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("UNSUPPORTED_MODEL")),
        "Orchestrator settings were copied across the Worker recipient boundary: {}",
        String::from_utf8_lossy(&copied_sender.stdout)
    );
    let partial = run_payload(json!({
        "hook_event_name":"PreToolUse",
        "tool_name":"codex_app__send_message_to_thread",
        "session_id":ORCHESTRATOR_ID,
        "tool_input":{"threadId":ORCHESTRATOR_ID,"model":"gpt-5.6-sol","thinking":"medium"}
    }))?;
    assert_denied_with(partial, "MISSING_IDENTITY")?;
    Ok(())
}

#[test]
fn orchestrator_delivery_rejects_stale_unrelated_and_mistyped_recipients() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("root.jsonl");
    std::fs::write(&transcript, root_transcript(ORCHESTRATOR_ID))?;
    for recipient in [OTHER, "stale-thread-id", "01a03690-a037-7141-af1c-1d7cdb09308"] {
        assert_denied_with(
            run_to(
                &transcript,
                ORCHESTRATOR_ID,
                recipient,
                "gpt-5.6-luna",
                "max",
            )?,
            "WRONG_RECIPIENT",
        )?;
    }
    Ok(())
}
