use super::active_wait;
use super::post_tool_use::{arm_binding, assert_no_cancellation, posttool_payload};
use super::{TestResult, binding_cancelled};
use serde_json::json;

// Failed or malformed sends, mismatches, and recipients without a wait are no-ops.
#[test]
fn unsuccessful_post_tool_sends_leave_other_bindings_armed() -> TestResult {
    let fixture = active_wait::trace_fixture()?;
    let recipient = arm_binding(&fixture, "recipient-thread", "recipient-turn", "wait-1")?;
    let unrelated = arm_binding(&fixture, "unrelated-thread", "unrelated-turn", "wait-2")?;

    let mut failed = posttool_payload("recipient-thread", "recipient-thread");
    failed["tool_response"]["isError"] = json!(true);
    assert_no_cancellation(&fixture, failed, &[&recipient, &unrelated])?;

    let mut malformed = posttool_payload("recipient-thread", "recipient-thread");
    malformed["tool_response"]["content"][0]["text"] = json!("not a JSON result");
    assert_no_cancellation(&fixture, malformed, &[&recipient, &unrelated])?;

    assert_no_cancellation(
        &fixture,
        posttool_payload("recipient-thread", "unrelated-thread"),
        &[&recipient, &unrelated],
    )?;
    assert_no_cancellation(
        &fixture,
        posttool_payload("unknown-thread", "unknown-thread"),
        &[&recipient, &unrelated],
    )?;

    let mut wrong_tool = posttool_payload("recipient-thread", "recipient-thread");
    wrong_tool["tool_name"] = json!("mcp__codex_app__other_tool");
    assert_no_cancellation(&fixture, wrong_tool, &[&recipient, &unrelated])?;

    assert_no_cancellation(
        &fixture,
        posttool_payload("recipient-thread", "recipient-thread"),
        &[&unrelated],
    )?;
    assert!(binding_cancelled(&fixture.state, &recipient));
    assert!(!binding_cancelled(&fixture.state, &unrelated));
    Ok(())
}

// A successful send must not guess between multiple live waits for one recipient.
#[test]
fn ambiguous_post_tool_recipient_is_a_no_op() -> TestResult {
    let fixture = active_wait::trace_fixture()?;
    let first = arm_binding(&fixture, "recipient-thread", "recipient-turn", "wait-1")?;
    let second = arm_binding(&fixture, "recipient-thread", "recipient-turn-2", "wait-2")?;

    assert_no_cancellation(
        &fixture,
        posttool_payload("recipient-thread", "recipient-thread"),
        &[&first, &second],
    )?;
    Ok(())
}

// The package contract keeps sender cancellation separate from preventive delivery hooks.
#[test]
fn sender_posttool_concern_is_separate_and_non_preventive() -> TestResult {
    let hooks_root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let hooks: serde_json::Value =
        serde_json::from_slice(&std::fs::read(hooks_root.join("hooks.json"))?)?;
    let posttool = hooks["hooks"]["PostToolUse"]
        .as_array()
        .ok_or("PostToolUse groups")?;
    assert_eq!(posttool.len(), 1);
    assert_eq!(
        posttool[0]["matcher"],
        "^mcp__codex_app__send_message_to_thread$"
    );

    let contract: serde_json::Value =
        serde_json::from_slice(&std::fs::read(hooks_root.join("capability-contract.json"))?)?;
    let concern = contract["concerns"]
        .as_array()
        .and_then(|concerns| {
            concerns.iter().find(|item| {
                item["concernId"] == "watcher-message-cancellation"
            })
        })
        .ok_or("sender cancellation concern")?;
    assert_eq!(concern["events"], json!(["PostToolUse"]));
    assert_eq!(concern["preventive"], false);
    assert_eq!(
        concern["inputContract"],
        "codexy.hooks.watcher-message-cancellation.v1"
    );
    Ok(())
}
