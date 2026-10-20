//! Covers authenticated delivery settings for both message directions.

use std::path::Path;

use serde_json::json;

use super::super::thread_delivery_support::{
    CHILD as WORKER_ID, PARENT as ORCHESTRATOR_ID, assert_admitted, assert_denied, input,
    plugin_roots, route, run,
};
use crate::support::TestResult;

// Keep host-authenticated settings separate from message args to detect mismatches.
fn run_delivery(
    root: &Path,
    event: &str,
    transcript: &Path,
    route_spec: (&str, &str, &str, &str, &str),
    recipient: &str,
    args: (&str, &str),
    prompt: &str,
) -> TestResult<std::process::Output> {
    let (direction, sender, target, model, thinking) = route_spec;
    run(
        root,
        event,
        transcript,
        Some(sender),
        input(recipient, args.0, args.1, prompt),
        Some(route(direction, sender, target, model, thinking)),
    )
}

#[test]
fn both_worker_pairs_and_parent_route_preserve_authenticated_settings() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    let admitted = [
        (
            "root_to_child",
            ORCHESTRATOR_ID,
            WORKER_ID,
            "gpt-6-luna",
            "max",
        ),
        (
            "root_to_child",
            ORCHESTRATOR_ID,
            WORKER_ID,
            "gpt-6.1-sol",
            "medium",
        ),
        (
            "child_to_parent",
            WORKER_ID,
            ORCHESTRATOR_ID,
            "gpt-6-astra",
            "medium",
        ),
    ];

    for event in ["PermissionRequest", "PreToolUse"] {
        for root in &roots {
            for (direction, sender, target, model, thinking) in admitted {
                assert_admitted(
                    run_delivery(
                        root,
                        event,
                        &transcript,
                        (direction, sender, target, model, thinking),
                        target,
                        (model, thinking),
                        "Post-result receipt wording is ordinary message content.",
                    )?,
                    event,
                    direction,
                )?;
            }

            for (direction, sender, target, model, thinking) in [
                (
                    "root_to_child",
                    ORCHESTRATOR_ID,
                    WORKER_ID,
                    "gpt-6-luna",
                    "medium",
                ),
                (
                    "root_to_child",
                    ORCHESTRATOR_ID,
                    WORKER_ID,
                    "gpt-6.1-sol",
                    "max",
                ),
                (
                    "root_to_child",
                    ORCHESTRATOR_ID,
                    WORKER_ID,
                    "gpt-6-sol",
                    "medium",
                ),
                (
                    "child_to_parent",
                    WORKER_ID,
                    ORCHESTRATOR_ID,
                    "gpt-6-luna",
                    "max",
                ),
            ] {
                assert_denied(
                    run_delivery(
                        root,
                        event,
                        &transcript,
                        (direction, sender, target, model, thinking),
                        target,
                        (model, thinking),
                        "receipt",
                    )?,
                    event,
                    "MISMATCHED_ROUTING_METADATA",
                )?;
            }

            assert_denied(
                run_delivery(
                    root,
                    event,
                    &transcript,
                    (
                        "root_to_child",
                        ORCHESTRATOR_ID,
                        WORKER_ID,
                        "gpt-6.1-sol",
                        "medium",
                    ),
                    WORKER_ID,
                    ("gpt-6-luna", "max"),
                    "receipt",
                )?,
                event,
                "UNSUPPORTED_MODEL",
            )?;
        }
    }
    Ok(())
}

#[test]
fn recipient_delivery_requires_model_and_thinking_even_with_valid_metadata() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    let metadata = route(
        "root_to_child",
        ORCHESTRATOR_ID,
        WORKER_ID,
        "gpt-6-luna",
        "max",
    );

    for root in &roots {
        for (tool_input, expected) in [
            (
                json!({"threadId":WORKER_ID,"thinking":"max"}),
                "MISSING_MODEL",
            ),
            (
                json!({"threadId":WORKER_ID,"model":"gpt-6-luna"}),
                "MISSING_THINKING",
            ),
            (
                json!({"threadId":WORKER_ID,"model":"","thinking":"max"}),
                "MISSING_MODEL",
            ),
            (
                json!({"threadId":WORKER_ID,"model":"gpt-6-luna","thinking":""}),
                "MISSING_THINKING",
            ),
        ] {
            assert_denied(
                run(
                    root,
                    "PreToolUse",
                    &transcript,
                    Some(ORCHESTRATOR_ID),
                    tool_input,
                    Some(metadata.clone()),
                )?,
                "PreToolUse",
                expected,
            )?;
        }
    }
    Ok(())
}
