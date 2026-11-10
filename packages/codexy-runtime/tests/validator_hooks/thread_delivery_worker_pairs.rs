//! Requires authenticated Worker delivery to match the creation allowlist.

use super::thread_delivery_support::{
    CHILD as WORKER_ID, PARENT as ORCHESTRATOR_ID, assert_admitted, assert_denied, input,
    plugin_roots, route, run_with_tool,
};
use crate::support::TestResult;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];
const MESSAGE_TOOLS: &[&str] = &[
    "codex_app__send_message_to_thread",
    "mcp__codex_app__send_message_to_thread",
];

#[test]
fn both_assigned_worker_pairs_admit_for_each_tool_and_hook() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;

    for root in &roots {
        for event in EVENTS {
            for tool in MESSAGE_TOOLS {
                for (model, thinking) in [
                    ("gpt-6-luna", "max"),
                    ("gpt-6.1-terra", "medium"),
                ] {
                    assert_admitted(
                        run_with_tool(
                            root,
                            event,
                            tool,
                            &transcript,
                            Some(ORCHESTRATOR_ID),
                            input(WORKER_ID, model, thinking, "Continue the assigned lane."),
                            Some(route(
                                "root_to_child",
                                ORCHESTRATOR_ID,
                                WORKER_ID,
                                model,
                                thinking,
                            )),
                        )?,
                        event,
                        "assigned Worker pair",
                    )?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn authenticated_existing_sol_medium_worker_is_denied() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;

    for root in &roots {
        for event in EVENTS {
            for tool in MESSAGE_TOOLS {
                assert_denied(
                    run_with_tool(
                        root,
                        event,
                        tool,
                        &transcript,
                        Some(ORCHESTRATOR_ID),
                        input(WORKER_ID, "gpt-6.1-sol", "medium", "Continue the assigned lane."),
                        Some(route(
                            "root_to_child",
                            ORCHESTRATOR_ID,
                            WORKER_ID,
                            "gpt-6.1-sol",
                            "medium",
                        )),
                    )?,
                    event,
                    "MISMATCHED_ROUTING_METADATA",
                )?;
            }
        }
    }
    Ok(())
}

#[test]
fn unsupported_model_ids_and_efforts_are_denied_for_assigned_worker() -> TestResult {
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
        for event in EVENTS {
            for (model, thinking, expected) in [
                ("gpt-6.1-luna", "max", "UNSUPPORTED_MODEL"),
                ("gpt-6-luna", "high", "UNSUPPORTED_THINKING"),
            ] {
                assert_denied(
                    run_with_tool(
                        root,
                        event,
                        "codex_app__send_message_to_thread",
                        &transcript,
                        Some(ORCHESTRATOR_ID),
                        input(WORKER_ID, model, thinking, "receipt"),
                        Some(metadata.clone()),
                    )?,
                    event,
                    expected,
                )?;
            }
        }
    }
    Ok(())
}
