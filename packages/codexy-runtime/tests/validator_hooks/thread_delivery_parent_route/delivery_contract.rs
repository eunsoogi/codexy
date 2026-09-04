use serde_json::json;

use super::fixtures::*;
use crate::support::TestResult;

#[test]
fn preventive_events_and_tool_aliases_bind_both_recipient_directions() -> TestResult {
    let temp = tempfile::tempdir()?;
    let child = temp.path().join("child.jsonl");
    let root_path = temp.path().join("root.jsonl");
    std::fs::write(&child, child_transcript(CHILD, &[PARENT]))?;
    std::fs::write(&root_path, root_transcript(PARENT))?;
    let installed = temp.path().join("installed");
    crate::support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &installed,
    )?;
    let authoritative = codexy_runtime::paths::repository_root().join("plugins/codexy");

    for root in [&authoritative, &installed] {
        for event in ["PermissionRequest", "PreToolUse"] {
            for tool in TOOLS {
                assert_admitted(run_route_at(
                    root,
                    &root_path,
                    PARENT,
                    CHILD,
                    "gpt-5.6-luna",
                    "max",
                    tool,
                    event,
                )?)?;
                assert_denied_with(
                    run_route_at(
                        root,
                        &root_path,
                        PARENT,
                        CHILD,
                        "gpt-5.6-sol",
                        "medium",
                        tool,
                        event,
                    )?,
                    "EXPECTED_CHILD_SETTINGS",
                )?;
                assert_admitted(run_route_at(
                    root,
                    &child,
                    CHILD,
                    PARENT,
                    "gpt-5.6-sol",
                    "medium",
                    tool,
                    event,
                )?)?;
                assert_denied_with(
                    run_route_at(
                        root,
                        &child,
                        CHILD,
                        PARENT,
                        "gpt-5.6-luna",
                        "max",
                        tool,
                        event,
                    )?,
                    "EXPECTED_PARENT_SETTINGS",
                )?;
            }
        }
    }
    Ok(())
}

#[test]
fn recipient_delivery_requires_explicit_model_and_thinking() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("root.jsonl");
    std::fs::write(&transcript, root_transcript(PARENT))?;
    for tool_input in [
        json!({"threadId":CHILD,"thinking":"max"}),
        json!({"threadId":CHILD,"model":"gpt-5.6-luna"}),
        json!({"threadId":CHILD,"model":"","thinking":"max"}),
        json!({"threadId":CHILD,"model":"gpt-5.6-luna","thinking":""}),
    ] {
        assert_denied_with(
            run_payload(json!({
                "hook_event_name":"PreToolUse",
                "tool_name":"codex_app__send_message_to_thread",
                "session_id":PARENT,
                "transcript_path":transcript,
                "tool_input":tool_input
            }))?,
            "MODEL_THINKING_REQUIRED",
        )?;
    }
    Ok(())
}

#[test]
fn post_result_receipts_require_one_stable_transition_key() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("child.jsonl");
    let transition = "idle-wait|878|28aac223";
    let prior = format!(
        "Post-result receipt for transition key={transition}; operation=update_goal(status=complete); exact tool result=complete"
    );
    let mut records = child_transcript(CHILD, &[PARENT]);
    records.extend_from_slice(&completed_delivery(
        PARENT,
        "gpt-5.6-sol",
        "medium",
        &prior,
    ));
    std::fs::write(&transcript, records)?;

    assert_denied_with(
        run_route_prompt(
            &transcript,
            CHILD,
            PARENT,
            "gpt-5.6-sol",
            "medium",
            &format!(
                "Post-result receipt for transition key={transition}; operation=update_goal(status=complete); exact tool result=complete; unchanged=true"
            ),
        )?,
        "DUPLICATE_DELIVERY",
    )?;
    assert_denied_with(
        run_route_prompt(
            &transcript,
            CHILD,
            PARENT,
            "gpt-5.6-sol",
            "medium",
            "#878 post-result receipt: update_goal(status=complete) succeeded; stable fingerprint=issue878|head-28aac223|gate-145-main-integration",
        )?,
        "DELIVERY_KEY_REQUIRED",
    )?;
    Ok(())
}
