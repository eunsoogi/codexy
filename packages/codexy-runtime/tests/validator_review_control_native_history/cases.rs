use serde_json::json;

use super::{TestResult, fixtures, native, native_history, rejected};

#[test]
fn helper_role_and_prompt_evidence_are_preserved() -> TestResult {
    let mut role = native::request();
    role["owner"]["pages"][1]["turns"][0]["items"][0]["receiver_agents"][0]["agent_role"] =
        json!("codexy-cartographer");
    let error = rejected(native_history::normalize_native_history(&role));
    assert!(error.contains("receiver agent role"));

    let receipt = native_history::normalize_native_history(&native::request())?;
    assert_eq!(
        receipt["owner"]["invocation"]["prompt"],
        "Review the current change using the selected profile."
    );
    assert!(receipt["owner"]["invocation"].get("purpose").is_none());
    Ok(())
}

#[test]
fn nested_page_cursor_identity_is_required() -> TestResult {
    let mut wrong = fixtures::request();
    wrong["reviewer"]["pages"][1]["page"]["cursor"] = json!("wrong-cursor");
    let error = rejected(native_history::normalize_native_history(&wrong));
    assert!(error.contains("page cursor"));

    let mut missing = fixtures::request();
    missing["reviewer"]["pages"][1]["page"]
        .as_object_mut()
        .ok_or("page metadata")?
        .remove("cursor");
    let error = rejected(native_history::normalize_native_history(&missing));
    assert!(error.contains("page cursor"));
    Ok(())
}

#[test]
fn dropped_newest_pages_are_rejected() -> TestResult {
    for role in ["owner", "reviewer"] {
        let mut request = fixtures::request();
        request[role]["pages"]
            .as_array_mut()
            .ok_or("pages")?
            .remove(0);
        let error = rejected(native_history::normalize_native_history(&request));
        assert!(error.contains("first page cursor"), "{role}: {error}");
    }
    Ok(())
}

#[test]
fn null_or_absent_first_cursor_preserves_complete_history() -> TestResult {
    for absent in [false, true] {
        let mut request = fixtures::request();
        for role in ["owner", "reviewer"] {
            let page = request[role]["pages"][0]["page"]
                .as_object_mut()
                .ok_or("page")?;
            if absent {
                page.remove("cursor");
            } else {
                page.insert("cursor".into(), serde_json::Value::Null);
            }
        }
        let receipt = native_history::normalize_native_history(&request)?;
        assert_eq!(receipt["history_projection"]["terminal_review_count"], 2);
    }
    Ok(())
}

#[test]
fn duplicate_and_ambiguous_events_are_rejected() -> TestResult {
    let mut duplicate = fixtures::request();
    duplicate["reviewer"]["pages"][1]["turns"][0]["items"][0]["id"] = json!("message-delta");
    let error = rejected(native_history::normalize_native_history(&duplicate));
    assert!(error.contains("duplicate event identity"));

    let mut tied = fixtures::request();
    tied["reviewer"]["pages"][1]["turns"][0]["completedAt"] = json!(200);
    let error = rejected(native_history::normalize_native_history(&tied));
    assert!(error.contains("ambiguous completion order"));
    Ok(())
}
