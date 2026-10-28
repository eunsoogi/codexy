//! Covers the nonblocking legacy Worker advisory and event-native denials.

use serde_json::{Value, json};

use super::thread_delivery_support::{
    CHILD as WORKER_ID, OTHER, PARENT as ORCHESTRATOR_ID, assert_admitted,
    assert_deprecated_context, assert_denied, input, plugin_roots, route, run, run_with_tool,
};
use crate::support::TestResult;

const MESSAGE_TOOLS: &[&str] = &[
    "codex_app__send_message_to_thread",
    "mcp__codex_app__send_message_to_thread",
];

#[test]
fn existing_legacy_worker_warning_is_identical_for_both_tools_and_launchers() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;

    for root in &roots {
        for tool in MESSAGE_TOOLS {
            let request = input(
                WORKER_ID,
                "gpt-6.1-sol",
                "medium",
                "Continue the existing lane.",
            );
            let metadata = route(
                "root_to_child",
                ORCHESTRATOR_ID,
                WORKER_ID,
                "gpt-6.1-sol",
                "medium",
            );
            assert_deprecated_context(
                run_with_tool(
                    root,
                    "PreToolUse",
                    tool,
                    &transcript,
                    Some(ORCHESTRATOR_ID),
                    request.clone(),
                    Some(metadata.clone()),
                )?,
                "PreToolUse",
                tool,
            )?;
            assert_admitted(
                run_with_tool(
                    root,
                    "PermissionRequest",
                    tool,
                    &transcript,
                    Some(ORCHESTRATOR_ID),
                    request,
                    Some(metadata),
                )?,
                "PermissionRequest",
                "legacy Worker permission request",
            )?;
        }
    }
    Ok(())
}

#[test]
fn invalid_routes_keep_only_their_existing_denials() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    let valid_legacy = route(
        "root_to_child",
        ORCHESTRATOR_ID,
        WORKER_ID,
        "gpt-6.1-sol",
        "medium",
    );
    let cases: [(Value, Value, &str); 6] = [
        (
            input(WORKER_ID, "gpt-6.1-sol", "medium", "receipt"),
            json!({"authenticated": true}),
            "MALFORMED_ROUTING_METADATA",
        ),
        (
            json!({"threadId": WORKER_ID, "thinking": "medium", "prompt": "receipt"}),
            valid_legacy.clone(),
            "MISSING_MODEL",
        ),
        (
            json!({"threadId": WORKER_ID, "model": "gpt-6.1-sol", "prompt": "receipt"}),
            valid_legacy.clone(),
            "MISSING_THINKING",
        ),
        (
            input(WORKER_ID, "gpt-6.1-sol", "medium", "receipt"),
            route(
                "root_to_child",
                OTHER,
                WORKER_ID,
                "gpt-6.1-sol",
                "medium",
            ),
            "MISMATCHED_ROUTING_METADATA",
        ),
        (
            input(OTHER, "gpt-6.1-sol", "medium", "receipt"),
            valid_legacy.clone(),
            "WRONG_RECIPIENT",
        ),
        (
            input(WORKER_ID, "gpt-6-luna", "max", "receipt"),
            valid_legacy,
            "UNSUPPORTED_MODEL",
        ),
    ];

    for root in &roots {
        for (tool_input, metadata, expected) in &cases {
            assert_denied(
                run(
                    root,
                    "PreToolUse",
                    &transcript,
                    Some(ORCHESTRATOR_ID),
                    tool_input.clone(),
                    Some(metadata.clone()),
                )?,
                "PreToolUse",
                expected,
            )?;
        }
        assert_denied(
            run_with_tool(
                root,
                "PreToolUse",
                "codex_app__create_thread",
                &transcript,
                Some(ORCHESTRATOR_ID),
                input(WORKER_ID, "gpt-6.1-sol", "medium", "receipt"),
                Some(route(
                    "root_to_child",
                    ORCHESTRATOR_ID,
                    WORKER_ID,
                    "gpt-6.1-sol",
                    "medium",
                )),
            )?,
            "PreToolUse",
            "ENVELOPE",
        )?;
    }
    Ok(())
}
