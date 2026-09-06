use serde_json::{Value, json};

use super::fixtures::{DELTA_HEAD, FULL_HEAD};

#[test]
fn terminal_handoff_markdown_forms_are_lossless() -> super::TestResult {
    let (request, full_text, delta_text) = request();
    let receipt = super::native_history::normalize_native_history(&request)?;
    let events = receipt["events"].as_array().ok_or("events")?;
    assert_eq!(events.len(), 2);

    assert_eq!(events[0]["kind"], "full");
    assert_eq!(events[0]["reviewed_head"], FULL_HEAD);
    assert_eq!(events[0]["terminal_result"], "UNOBSERVABLE");
    assert_eq!(events[0]["raw_text"], full_text);
    assert_eq!(events[1]["kind"], "delta");
    assert_eq!(events[1]["reviewed_head"], DELTA_HEAD);
    assert_eq!(events[1]["terminal_result"], "BLOCK");
    assert_eq!(events[1]["raw_text"], delta_text);

    let kind_span = &events[1]["field_sources"]["spans"]["spans"]["kind"];
    assert_eq!(kind_span["source"], "pages[0].turns[0].items[0].content");
    assert_eq!(kind_span["start"], 0);
    assert_eq!(kind_span["end"], 62);

    for event in events {
        let text = event["raw_text"].as_str().ok_or("raw text")?;
        for finding in event["findings"].as_array().ok_or("findings")? {
            let span = &finding["source_span"]["raw"];
            let start = span["start"].as_u64().ok_or("start")? as usize;
            let end = span["end"].as_u64().ok_or("end")? as usize;
            assert_eq!(
                &text[start..end],
                finding["text"].as_str().ok_or("finding text")?
            );
        }
    }
    Ok(())
}

#[test]
fn generic_result_labels_outside_terminal_handoff_fail_closed() {
    let (mut request, full_text, _) = request();
    let text = full_text.replace("- Result: **UNOBSERVABLE**", "").replace(
        "## Terminal handoff",
        "Result: **PASS**\n\n## Terminal handoff",
    );
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(
        error.contains("terminal result"),
        "unexpected error: {error}"
    );
}

#[test]
fn contradictory_terminal_handoff_heads_are_rejected() {
    let (mut request, full_text, _) = request();
    let extra =
        format!("- Exact current PR head: `{FULL_HEAD}`\n- Exact current head: `{DELTA_HEAD}`");
    let text = full_text.replace(&format!("- Exact current PR head: `{FULL_HEAD}`"), &extra);
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(error.contains("reviewed head is contradictory"), "{error}");
}

#[test]
fn contradictory_terminal_handoff_results_are_rejected() {
    let (mut request, full_text, _) = request();
    let text = full_text.replace(
        "- Result: **UNOBSERVABLE**",
        "- Result: **UNOBSERVABLE**\n- Terminal result: BLOCK",
    );
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(
        error.contains("terminal result is contradictory"),
        "{error}"
    );
}

#[test]
fn contradictory_request_kind_is_rejected() {
    let (mut request, _, _) = request();
    request["reviewer"]["pages"][1]["turns"][0]["review"] = json!({"kind": "delta"});
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(error.contains("review kind is contradictory"), "{error}");
}

fn request() -> (Value, String, String) {
    let mut request = super::fixtures::request();
    let delta_prompt = json!({
        "type": "userMessage",
        "id": "native-delta-request",
        "content": [{
            "type": "text",
            "text": "Authorized strict delta recheck on the frozen final head only."
        }]
    });
    let full_prompt = json!({
        "type": "userMessage",
        "id": "native-full-request",
        "content": [{
            "type": "text",
            "text": "Perform the selected strict-profile review for issue #985 and Draft PR #983."
        }]
    });
    let delta_text = terminal_handoff(DELTA_HEAD, "delta", "BLOCK", "Exact current head");
    let full_text = terminal_handoff(FULL_HEAD, "full", "UNOBSERVABLE", "Exact current PR head");
    request["reviewer"]["pages"][0]["turns"][0]["items"] = json!([
        delta_prompt,
        json!({
            "type": "agentMessage",
            "id": "native-delta-message",
            "phase": "final_answer",
            "text": delta_text.clone()
        })
    ]);
    request["reviewer"]["pages"][1]["turns"][0]["items"] = json!([
        full_prompt,
        json!({
            "type": "agentMessage",
            "id": "native-full-message",
            "phase": "final_answer",
            "text": full_text.clone()
        })
    ]);
    (request, full_text, delta_text)
}

fn terminal_handoff(head: &str, label: &str, result: &str, head_label: &str) -> String {
    format!(
        "## Blocking findings\n\n- **HIGH — {label} source issue (`in_scope_blocker`)**: preserve this exact finding.\n\n[src/{label}.rs](https://example.test/src/{label}.rs)\n\n## Terminal handoff\n\n- {head_label}: `{head}`\n- {}: **{result}**\n\n> - Exact current head: `0000000000000000000000000000000000000000`\n\n```text\n- Result: **PASS**\n```\n",
        if result == "BLOCK" {
            "Terminal result"
        } else {
            "Result"
        }
    )
}
