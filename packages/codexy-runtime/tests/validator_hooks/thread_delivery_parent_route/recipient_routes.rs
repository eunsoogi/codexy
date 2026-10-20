//! Exercises authenticated Worker recipient pairs through hook subprocesses.

use serde_json::json;

use super::super::thread_delivery_support::{
    CHILD as WORKER_ID, OTHER, PARENT as ORCHESTRATOR_ID, assert_admitted, assert_denied, input,
    plugin_roots, route, run,
};
use crate::support::TestResult;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

#[test]
fn assigned_worker_pairs_admit_on_authoritative_and_installed_hooks() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    let pairs = [("gpt-6-luna", "max"), ("gpt-6.1-sol", "medium")];

    for event in EVENTS {
        for root in &roots {
            for (model, thinking) in pairs {
                assert_admitted(
                    run(
                        root,
                        event,
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
                    "Orchestrator-to-Worker",
                )?;
            }
        }
    }
    Ok(())
}

#[test]
fn recipient_pair_rejects_crossed_settings_and_argument_mismatch() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;

    for event in EVENTS {
        for root in &roots {
            for (model, thinking) in [("gpt-6-luna", "medium"), ("gpt-6.1-sol", "max")] {
                assert_denied(
                    run(
                        root,
                        event,
                        &transcript,
                        Some(ORCHESTRATOR_ID),
                        input(WORKER_ID, model, thinking, "receipt"),
                        Some(route(
                            "root_to_child",
                            ORCHESTRATOR_ID,
                            WORKER_ID,
                            model,
                            thinking,
                        )),
                    )?,
                    event,
                    "MISMATCHED_ROUTING_METADATA",
                )?;
            }
            assert_denied(
                run(
                    root,
                    event,
                    &transcript,
                    Some(ORCHESTRATOR_ID),
                    input(WORKER_ID, "gpt-6-luna", "max", "receipt"),
                    Some(route(
                        "root_to_child",
                        ORCHESTRATOR_ID,
                        WORKER_ID,
                        "gpt-6.1-sol",
                        "medium",
                    )),
                )?,
                event,
                "UNSUPPORTED_MODEL",
            )?;
            assert_denied(
                run(
                    root,
                    event,
                    &transcript,
                    Some(ORCHESTRATOR_ID),
                    input(OTHER, "gpt-6.1-sol", "medium", "receipt"),
                    Some(route(
                        "root_to_child",
                        ORCHESTRATOR_ID,
                        WORKER_ID,
                        "gpt-6.1-sol",
                        "medium",
                    )),
                )?,
                event,
                "WRONG_RECIPIENT",
            )?;
        }
    }
    Ok(())
}

#[test]
fn metadata_absence_keeps_the_legacy_noop_separate() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;

    for root in &roots {
        assert_admitted(
            run(
                root,
                "PreToolUse",
                &transcript,
                Some(ORCHESTRATOR_ID),
                input(WORKER_ID, "gpt-6.1-sol", "medium", "legacy"),
                None,
            )?,
            "PreToolUse",
            "legacy-metadata-absent",
        )?;
    }
    Ok(())
}

#[test]
fn recipient_route_rejects_wrong_senders_and_malformed_envelopes() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;

    for event in EVENTS {
        for root in &roots {
            assert_denied(
                run(
                    root,
                    event,
                    &transcript,
                    Some(ORCHESTRATOR_ID),
                    input(WORKER_ID, "gpt-6.1-sol", "medium", "receipt"),
                    Some(route(
                        "root_to_child",
                        OTHER,
                        WORKER_ID,
                        "gpt-6.1-sol",
                        "medium",
                    )),
                )?,
                event,
                "MISMATCHED_ROUTING_METADATA",
            )?;
            assert_denied(
                run(
                    root,
                    event,
                    &transcript,
                    Some(ORCHESTRATOR_ID),
                    input(WORKER_ID, "gpt-6.1-sol", "medium", "receipt"),
                    Some(json!({"authenticated":true})),
                )?,
                event,
                "MALFORMED_ROUTING_METADATA",
            )?;
        }
    }
    Ok(())
}
