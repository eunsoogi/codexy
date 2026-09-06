use serde_json::json;

use super::fixtures::FULL_HEAD;

#[test]
fn negated_affirmative_request_kind_is_rejected() {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] =
        json!("Do not perform the selected strict-profile review.");
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(error.contains("review kind"), "{error}");
}

#[test]
fn unavailable_reviewed_head_does_not_fall_through_to_a_later_sha() {
    let (mut request, full_text, _) = super::native_forms::request();
    let text = full_text.replace(
        &format!("- Exact current PR head: `{FULL_HEAD}`"),
        "- Reviewed head: unavailable; later GitHub head `0000000000000000000000000000000000000000`",
    );
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(error.contains("reviewed head"), "{error}");
}

#[test]
fn reviewed_head_span_excludes_markdown_delimiters() -> super::TestResult {
    let (request, _, _) = super::native_forms::request();
    let receipt = super::native_history::normalize_native_history(&request)?;
    let event = receipt["events"]
        .as_array()
        .and_then(|events| events.iter().find(|event| event["kind"] == "full"))
        .ok_or("full event")?;
    let text = event["raw_text"].as_str().ok_or("raw text")?;
    let span = &event["field_sources"]["spans"]["spans"]["reviewed_head"];
    let start = span["start"].as_u64().ok_or("head span start")? as usize;
    let end = span["end"].as_u64().ok_or("head span end")? as usize;
    assert_eq!(&text[start..end], FULL_HEAD);
    Ok(())
}

#[test]
fn mixed_fence_markers_do_not_reopen_findings() -> super::TestResult {
    let (mut request, full_text, _) = super::native_forms::request();
    let text = format!(
        "{full_text}\n```text\n~~~\n- **HIGH — fake source issue (`in_scope_blocker`)**: ignore this.\n~~~\n```\n"
    );
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    let receipt = super::native_history::normalize_native_history(&request)?;
    let event = receipt["events"]
        .as_array()
        .and_then(|events| events.iter().find(|event| event["kind"] == "full"))
        .ok_or("full event")?;
    assert_eq!(event["findings"].as_array().ok_or("findings")?.len(), 1);
    Ok(())
}
