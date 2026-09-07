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
fn polite_prefix_negation_is_rejected() {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] =
        json!("Please do not perform the selected strict-profile review.");
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

#[test]
fn quoted_affirmative_request_kind_is_rejected() {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] = json!(
        "\"Perform the selected strict-profile review.\" This sentence is only a quoted example."
    );
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(error.contains("review kind"), "{error}");
}

#[test]
fn post_negated_affirmative_request_kind_is_rejected() {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] =
        json!("Perform the selected strict-profile review? Do not.");
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(error.contains("review kind"), "{error}");
}

#[test]
fn independent_prefix_no_edit_does_not_negate_request_kind() -> super::TestResult {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] =
        json!("Please do not edit files; perform the selected strict-profile review.");
    let receipt = super::native_history::normalize_native_history(&request)?;
    assert!(
        receipt["events"]
            .as_array()
            .is_some_and(|events| { events.iter().any(|event| event["kind"] == "full") })
    );
    Ok(())
}

#[test]
fn independent_prefix_action_does_not_negate_request_kind() -> super::TestResult {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] =
        json!("Please do not run tests; perform the selected strict-profile review.");
    let receipt = super::native_history::normalize_native_history(&request)?;
    assert!(
        receipt["events"]
            .as_array()
            .is_some_and(|events| { events.iter().any(|event| event["kind"] == "full") })
    );
    Ok(())
}

#[test]
fn bold_quoted_request_example_is_rejected() {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] = json!(
        "**\"Perform the selected strict-profile review.\"** This sentence is only an example."
    );
    let error = super::rejected(super::native_history::normalize_native_history(&request));
    assert!(error.contains("review kind"), "{error}");
}

#[test]
fn nested_terminal_heading_preserves_parent_result_label() -> super::TestResult {
    let (mut request, full_text, _) = super::native_forms::request();
    let text = full_text.replace(
        "- Result: **UNOBSERVABLE**",
        "### Documentation\n\nThe terminal handoff documentation.\n\n- Result: **UNOBSERVABLE**",
    );
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    let receipt = super::native_history::normalize_native_history(&request)?;
    let event = receipt["events"]
        .as_array()
        .and_then(|events| events.iter().find(|event| event["kind"] == "full"))
        .ok_or("full event")?;
    assert_eq!(event["terminal_result"], "UNOBSERVABLE");
    Ok(())
}

#[test]
fn nested_followup_heading_preserves_parent_observation() -> super::TestResult {
    let mut request = super::markdown::request(false);
    let original = request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"]
        .as_str()
        .ok_or("markdown text")?;
    let text = original.replace(
        "## Non-blocking follow-up\n",
        "## Non-blocking follow-up\n\n### Documentation\n\n",
    );
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    let receipt = super::native_history::normalize_native_history(&request)?;
    let event = receipt["events"]
        .as_array()
        .and_then(|events| events.iter().find(|event| event["kind"] == "full"))
        .ok_or("full event")?;
    assert!(event["findings"].as_array().is_some_and(|findings| {
        findings
            .iter()
            .any(|finding| finding["disposition"] == "out_of_scope_followup")
    }));
    Ok(())
}

#[test]
fn inline_code_in_affirmative_request_kind_is_operative() -> super::TestResult {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] =
        json!("Perform the selected strict-profile review of `request.rs`.");
    let receipt = super::native_history::normalize_native_history(&request)?;
    assert!(
        receipt["events"]
            .as_array()
            .is_some_and(|events| { events.iter().any(|event| event["kind"] == "full") })
    );
    Ok(())
}

#[test]
fn no_edits_clause_in_affirmative_request_kind_is_operative() -> super::TestResult {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] =
        json!("Perform the selected strict-profile review with no edits.");
    let receipt = super::native_history::normalize_native_history(&request)?;
    assert!(
        receipt["events"]
            .as_array()
            .is_some_and(|events| { events.iter().any(|event| event["kind"] == "full") })
    );
    Ok(())
}

#[test]
fn independent_no_edit_sentence_in_affirmative_request_kind_is_operative() -> super::TestResult {
    let (mut request, _, _) = super::native_forms::request();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["content"][0]["text"] =
        json!("Perform the selected strict-profile review. Do not edit files.");
    let receipt = super::native_history::normalize_native_history(&request)?;
    assert!(
        receipt["events"]
            .as_array()
            .is_some_and(|events| { events.iter().any(|event| event["kind"] == "full") })
    );
    Ok(())
}

#[test]
fn same_marker_content_is_not_a_fence_closer() -> super::TestResult {
    let (mut request, full_text, _) = super::native_forms::request();
    let text = format!(
        "{full_text}\n```text\n- **HIGH — fake source issue (`in_scope_blocker`)**: ignore this.\n```not-a-close\n- **HIGH — second fake source (`in_scope_blocker`)**: ignore this too.\n```\n"
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

#[test]
fn fenced_heading_does_not_truncate_finding_text() -> super::TestResult {
    let (mut request, full_text, _) = super::native_forms::request();
    let text = full_text.replace(
        "- **HIGH — full source issue (`in_scope_blocker`)**: preserve this exact finding.",
        "- **HIGH — full source issue (`in_scope_blocker`)**: preserve this exact finding.\n\n```text\n## fenced heading\ncontinuation inside the finding\n```",
    );
    request["reviewer"]["pages"][1]["turns"][0]["items"][1]["text"] = json!(text);
    let receipt = super::native_history::normalize_native_history(&request)?;
    let event = receipt["events"]
        .as_array()
        .and_then(|events| events.iter().find(|event| event["kind"] == "full"))
        .ok_or("full event")?;
    let finding = event["findings"]
        .as_array()
        .and_then(|findings| findings.first())
        .ok_or("finding")?;
    let finding_text = finding["text"].as_str().ok_or("finding text")?;
    assert!(finding_text.contains("## fenced heading"));
    assert!(finding_text.contains("continuation inside the finding"));
    Ok(())
}
