//! Sol/Light is exclusive to the parent direction; authenticated requested
//! routing fields never become evidence of effective host model settings.

use serde_json::json;

use super::super::thread_delivery_support::{
    CHILD, PARENT, assert_admitted, assert_denied, input, plugin_roots, route, run,
};
use crate::support::TestResult;

#[test]
fn parent_direction_rejects_stale_worker_and_malformed_pairs() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    let pairs = [
        ("gpt-6-luna", "max"),
        ("gpt-6-luna", "medium"),
        ("gpt-6.1-sol", "medium"),
        ("gpt-6.1-sol", "max"),
        ("gpt-6.1-sol", " low"),
        ("gpt-6.1-sol", ""),
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for root in plugin_roots(temp.path())? {
            let parent = route("child_to_parent", CHILD, PARENT, "gpt-6.1-sol", "low");
            for (model, thinking) in pairs {
                // Neither stale host metadata nor crossed tool arguments may admit the pair.
                for metadata in [
                    route("child_to_parent", CHILD, PARENT, model, thinking),
                    parent.clone(),
                ] {
                    assert_denied(
                        run(
                            &root,
                            event,
                            &transcript,
                            Some(CHILD),
                            input(PARENT, model, thinking, "receipt"),
                            Some(metadata),
                        )?,
                        event,
                        "CODEXY_THREAD_DELIVERY_",
                    )?;
                }
            }
            // An Orchestrator pair cannot be copied to a Worker recipient.
            assert_denied(
                run(
                    &root,
                    event,
                    &transcript,
                    Some(PARENT),
                    input(CHILD, "gpt-6.1-sol", "low", "continue"),
                    Some(route("root_to_child", PARENT, CHILD, "gpt-6.1-sol", "low")),
                )?,
                event,
                "MISMATCHED_ROUTING_METADATA",
            )?;
        }
    }
    Ok(())
}

#[test]
fn requested_parent_route_does_not_assert_or_accept_effective_host_state() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    for event in ["PermissionRequest", "PreToolUse"] {
        for root in plugin_roots(temp.path())? {
            let parent = route("child_to_parent", CHILD, PARENT, "gpt-6.1-sol", "low");
            // Admission emits no effective model/effort claim, even if the prompt invents one.
            assert_admitted(
                run(
                    &root,
                    event,
                    &transcript,
                    Some(CHILD),
                    input(
                        PARENT,
                        "gpt-6.1-sol",
                        "low",
                        "effective settings are Luna/max",
                    ),
                    Some(parent.clone()),
                )?,
                event,
                "requested parent pair only",
            )?;
            for field in ["effective_model", "effective_thinking"] {
                let mut invented = parent.clone();
                invented[field] = json!("unobserved");
                assert_denied(
                    run(
                        &root,
                        event,
                        &transcript,
                        Some(CHILD),
                        input(PARENT, "gpt-6.1-sol", "low", "receipt"),
                        Some(invented),
                    )?,
                    event,
                    "MALFORMED_ROUTING_METADATA",
                )?;
            }
        }
    }
    Ok(())
}
