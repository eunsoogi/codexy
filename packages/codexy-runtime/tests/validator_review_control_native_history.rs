use serde_json::{Value, json};

#[path = "validator_review_control_native_history/pure.rs"]
mod native_history;

#[path = "validator_review_control_native_history/cases.rs"]
mod cases;
#[path = "validator_review_control_native_history/fixtures.rs"]
mod fixtures;
#[path = "validator_review_control_native_history/live.rs"]
mod live;
#[path = "validator_review_control_native_history/markdown.rs"]
mod markdown;
#[path = "validator_review_control_native_history/native.rs"]
mod native;
#[path = "validator_review_control_native_history/native_forms.rs"]
mod native_forms;

type TestResult = Result<(), String>;

#[test]
fn complete_paginated_sources_project_losslessly() -> TestResult {
    let receipt = native_history::normalize_native_history(&fixtures::request())?;
    assert_eq!(receipt["schema"], "codexy.review-control-native-history.v1");
    assert_eq!(
        receipt["source"]["owner"]["pages"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        receipt["source"]["reviewer"]["pages"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        receipt["owner"]["invocation"]["receiver"],
        "reviewer-thread"
    );
    assert_eq!(receipt["owner"]["invocation"]["model"], "model.initial");
    assert_eq!(
        receipt["owner"]["helpers"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(receipt["history_projection"]["full_review_count"], 1);
    assert_eq!(receipt["history_projection"]["delta_review_count"], 1);
    assert_eq!(receipt["history_projection"]["terminal_review_count"], 2);
    assert_eq!(receipt["events"][0]["kind"], "full");
    assert_eq!(receipt["events"][0]["reviewed_head"], fixtures::FULL_HEAD);
    assert_eq!(
        receipt["events"][0]["reviewer"]["model"],
        "model.continuation"
    );
    assert_eq!(receipt["events"][0]["reviewer"]["source"], "reviewer_event");
    assert_eq!(receipt["events"][1]["kind"], "delta");
    assert_eq!(receipt["events"][1]["reviewer"]["model"], "model.initial");
    assert_eq!(receipt["events"][1]["reviewer"]["source"], "owner_spawn");
    assert_eq!(receipt["events"][1]["findings"][1]["path"], Value::Null);
    assert_eq!(
        receipt["events"][1]["findings"][1]["metadata"]["derived"],
        true
    );
    assert!(receipt["events"][1]["findings"][1]["source_span"]["path"].is_string());
    assert_eq!(receipt["admission"]["result"], "not_admitted");
    assert_eq!(receipt["admission"]["authentication"], "not_attested");
    Ok(())
}

#[test]
fn incomplete_page_chain_is_rejected() -> TestResult {
    let mut request = fixtures::request();
    for role in ["owner", "reviewer"] {
        let pages = request[role]["pages"]
            .as_array_mut()
            .ok_or_else(|| format!("{role} pages"))?;
        let _ = pages.pop();
        let error = rejected(native_history::normalize_native_history(&request));
        assert!(error.contains("pagination is incomplete"));
        request = fixtures::request();
    }
    Ok(())
}

#[test]
fn helper_substitution_and_contradictory_facts_are_rejected() -> TestResult {
    let mut substituted = fixtures::request();
    substituted["owner"]["pages"][1]["turns"][0]["items"][0]["receiverThreadIds"] =
        json!(["different-reviewer"]);
    let error = rejected(native_history::normalize_native_history(&substituted));
    assert!(error.contains("exactly one matching"));

    let mut contradictory = fixtures::request();
    contradictory["reviewer"]["pages"][0]["turns"][0]["items"][0]["review"]["terminal_result"] =
        json!("PASS");
    let error = rejected(native_history::normalize_native_history(&contradictory));
    assert!(error.contains("terminal result"));
    Ok(())
}

#[test]
fn current_snapshot_binding_is_temporal_and_non_admitting() -> TestResult {
    let receipt = native_history::normalize_native_history(&fixtures::request())?;
    let bound = native_history::bind_current_pr_snapshot(
        &receipt,
        &fixtures::current_snapshot(50, fixtures::CURRENT_HEAD),
    )?;
    assert_eq!(bound["binding"]["current_head"], fixtures::CURRENT_HEAD);
    assert_eq!(bound["admission"]["current_pr"], "bound");
    assert_eq!(bound["admission"]["temporal"], "proved_post_pr");
    assert_eq!(bound["admission"]["authentication"], "not_attested");
    assert_eq!(bound["admission"]["result"], "not_admitted");
    assert_eq!(bound["events"][0]["reviewed_head"], fixtures::FULL_HEAD);
    assert_eq!(bound["events"][1]["reviewed_head"], fixtures::DELTA_HEAD);

    let mut wrong = fixtures::current_snapshot(50, fixtures::CURRENT_HEAD);
    wrong["number"] = json!(24);
    let error = rejected(native_history::bind_current_pr_snapshot(&receipt, &wrong));
    assert!(error.contains("pull request"));
    Ok(())
}

#[test]
fn pre_creation_completion_is_rejected() -> TestResult {
    let receipt = native_history::normalize_native_history(&fixtures::request())?;
    let error = rejected(native_history::bind_current_pr_snapshot(
        &receipt,
        &fixtures::current_snapshot(100, fixtures::CURRENT_HEAD),
    ));
    assert!(error.contains("before or at PR creation"));
    Ok(())
}

#[test]
fn pathless_semantics_and_authentication_claims_fail_closed() -> TestResult {
    let mut pathless = fixtures::request();
    let finding = pathless["reviewer"]["pages"][0]["turns"][0]["items"][0]["review"]["findings"][1]
        .as_object_mut()
        .ok_or("pathless finding")?;
    let _ = finding.remove("semanticKind");
    finding.insert("disposition".into(), json!("resolved"));
    let receipt = native_history::normalize_native_history(&pathless)?;
    let finding = &receipt["events"][1]["findings"][1];
    assert_eq!(finding["path"], Value::Null);
    assert_eq!(finding["semantic_status"], "unclassified");
    assert_eq!(finding["metadata"]["unclassified"], true);
    assert_eq!(finding["disposition"], "resolved");
    assert_eq!(receipt["admission"]["result"], "not_admitted");

    let mut claimed = fixtures::request();
    claimed["authenticated"] = json!(true);
    let error = rejected(native_history::normalize_native_history(&claimed));
    assert!(error.contains("must not claim authentication"));
    Ok(())
}

#[test]
fn request_and_markdown_sources_preserve_derived_spans() -> TestResult {
    let receipt = native_history::normalize_native_history(&markdown::request(false))?;
    assert_eq!(receipt["events"].as_array().map(Vec::len), Some(2));
    assert_eq!(receipt["events"][0]["kind"], "full");
    assert_eq!(receipt["events"][0]["reviewed_head"], fixtures::FULL_HEAD);
    assert_eq!(receipt["events"][0]["reviewer"]["model"], "model.full");
    assert_eq!(receipt["events"][1]["kind"], "delta");
    assert_eq!(receipt["events"][1]["reviewed_head"], fixtures::DELTA_HEAD);
    assert_eq!(
        receipt["events"][1]["field_sources"]["spans"]["spans"]["kind"]["source"],
        "pages[0].turns[0].items[0].content"
    );
    let finding = &receipt["events"][0]["findings"][0];
    assert_eq!(finding["path"], "src/full.rs");
    assert_eq!(finding["metadata"]["derived"], true);
    assert!(finding["source_span"]["raw"]["start"].is_number());
    assert!(finding["source_span"]["raw"]["end"].is_number());
    let findings = receipt["events"][0]["findings"]
        .as_array()
        .ok_or("markdown findings")?;
    assert_eq!(findings.len(), 3);
    assert!(
        !findings[0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("sibling issue")
    );
    assert_eq!(findings[1]["path"], "src/sibling.rs");
    assert_eq!(findings[2]["path"], "plugins/followup.toml");
    assert_eq!(findings[2]["disposition"], "out_of_scope_followup");
    for pair in findings.windows(2) {
        assert!(
            pair[1]["source_span"]["raw"]["start"].as_u64()
                >= pair[0]["source_span"]["raw"]["end"].as_u64()
        );
    }

    let crlf = native_history::normalize_native_history(&markdown::request(true))?;
    let crlf_span = &crlf["events"][0]["findings"][0]["source_span"]["raw"];
    assert!(crlf_span["end"].as_u64() > crlf_span["start"].as_u64());
    Ok(())
}

#[test]
fn actual_host_markdown_without_structured_metadata_is_lossless() -> TestResult {
    let receipt = native_history::normalize_native_history(&markdown::actual_request())?;
    assert_eq!(receipt["events"].as_array().map(Vec::len), Some(2));
    assert_eq!(receipt["events"][0]["message_id"], "actual-message-full");
    assert_eq!(receipt["events"][0]["kind"], "full");
    assert_eq!(receipt["events"][0]["terminal_result"], "BLOCK");
    assert_eq!(receipt["events"][0]["reviewed_head"], fixtures::FULL_HEAD);
    assert_eq!(receipt["events"][1]["message_id"], "actual-message-delta");
    assert_eq!(receipt["events"][1]["kind"], "delta");
    assert_eq!(receipt["events"][1]["terminal_result"], "BLOCK");
    assert_eq!(receipt["events"][1]["reviewed_head"], fixtures::DELTA_HEAD);
    for event in receipt["events"].as_array().ok_or("events")? {
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
fn markdown_conflicting_operational_labels_are_rejected() -> TestResult {
    let error = rejected(native_history::normalize_native_history(
        &markdown::conflicting_terminal_request(),
    ));
    assert!(error.contains("markdown terminal result is contradictory"));
    Ok(())
}

#[test]
fn markdown_conflicting_finding_labels_are_rejected() {
    let error = rejected(native_history::normalize_native_history(
        &markdown::conflicting_finding_labels_request(),
    ));
    assert!(error.contains("semantic kind"));
}

fn rejected(result: Result<Value, String>) -> String {
    match result {
        Ok(_) => "unexpected success".into(),
        Err(error) => error,
    }
}
