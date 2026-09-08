use serde_json::{Value, json};

use super::fixtures::{CURRENT_HEAD, DELTA_HEAD, FULL_HEAD};

fn required_current_head_request() -> Value {
    let mut request = super::fixtures::request();
    request["reviewer"]["pages"][0]["turns"][0]["items"] = json!([
        {
            "type": "userMessage",
            "id": "current-head-request",
            "content": [{"type": "text", "text": "Read-only Sentinel review."}]
        },
        {
            "type": "agentMessage",
            "id": "current-head-message",
            "phase": "final_answer",
            "text": required_current_head_markdown()
        }
    ]);
    request["reviewer"]["pages"][1]["turns"][0]["items"] = json!([]);
    request
}

fn required_current_head_markdown() -> String {
    format!(
        "## Blocking findings\n\n1. **High — F1 remains unresolved: the witness is not bound to the recovered predecessor.** Disposition: `in_scope_blocker`; preserve the complete history.\n\n[src/review.rs:42](https://example.test/src/review.rs)\n\n## Terminal control\n\n`terminal_result = BLOCK`\n\n- `reviewed_head`: `{CURRENT_HEAD}`\n\nOrdered `terminal_review_history`:\n\n1. Full — BLOCK at `{FULL_HEAD}`.\n2. Delta — BLOCK at `{DELTA_HEAD}`.\n3. Required-current-head — BLOCK at `{CURRENT_HEAD}`."
    )
}

#[test]
fn actual_terminal_control_markdown_preserves_current_head_finding() -> super::TestResult {
    let receipt =
        super::native_history::normalize_native_history(&required_current_head_request())?;
    let event = receipt["events"].as_array().ok_or("events")?;
    assert_eq!(event.len(), 1);
    assert_eq!(event[0]["kind"], "required_current_head");
    assert_eq!(event[0]["terminal_result"], "BLOCK");
    assert_eq!(event[0]["reviewed_head"], CURRENT_HEAD);
    let finding = &event[0]["findings"][0];
    assert_eq!(finding["path"], "src/review.rs");
    assert_eq!(finding["disposition"], "in_scope_blocker");
    assert_eq!(finding["metadata"]["derived"], true);
    assert_ne!(finding["id"], "F1");
    assert!(!finding["id"].as_str().unwrap_or_default().contains("919"));
    let raw = event[0]["raw_text"].as_str().ok_or("raw text")?;
    let start = finding["source_span"]["raw"]["start"]
        .as_u64()
        .ok_or("finding start")? as usize;
    let end = finding["source_span"]["raw"]["end"]
        .as_u64()
        .ok_or("finding end")? as usize;
    assert_eq!(
        &raw[start..end],
        finding["text"].as_str().ok_or("finding text")?
    );
    Ok(())
}

#[test]
fn historical_pass_and_quoted_syntax_do_not_override_current_result() -> super::TestResult {
    let mut request = required_current_head_request();
    let text = request["reviewer"]["pages"][0]["turns"][0]["items"][1]["text"]
        .as_str()
        .ok_or("current markdown")?
        .to_owned()
        + "\n\n> `terminal_result = PASS`\n\n```text\n- `reviewed_head`: `"
        + FULL_HEAD
        + "`\nterminal_result = PASS\n```";
    request["reviewer"]["pages"][0]["turns"][0]["items"][1]["text"] = json!(text);
    let receipt = super::native_history::normalize_native_history(&request)?;
    assert_eq!(receipt["events"][0]["terminal_result"], "BLOCK");
    assert_eq!(receipt["events"][0]["reviewed_head"], CURRENT_HEAD);
    Ok(())
}

#[test]
fn current_terminal_result_and_head_contradictions_are_rejected() -> super::TestResult {
    let mut result = required_current_head_request();
    let original = result["reviewer"]["pages"][0]["turns"][0]["items"][1]["text"]
        .as_str()
        .ok_or("current markdown")?
        .to_owned();
    result["reviewer"]["pages"][0]["turns"][0]["items"][1]["text"] =
        json!(format!("{original}\n- Terminal result: PASS"));
    let error = super::rejected(super::native_history::normalize_native_history(&result));
    assert!(
        error.contains("markdown terminal result is contradictory"),
        "{error}"
    );

    let mut head = required_current_head_request();
    let original = head["reviewer"]["pages"][0]["turns"][0]["items"][1]["text"]
        .as_str()
        .ok_or("current markdown")?
        .to_owned();
    head["reviewer"]["pages"][0]["turns"][0]["items"][1]["text"] =
        json!(format!("{original}\n- `reviewed_head`: `{FULL_HEAD}`"));
    let error = super::rejected(super::native_history::normalize_native_history(&head));
    assert!(
        error.contains("markdown reviewed head is contradictory"),
        "{error}"
    );
    Ok(())
}
