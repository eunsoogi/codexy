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
fn missing_receiver_role_evidence_is_rejected() -> TestResult {
    let mut missing = native::request();
    missing["owner"]["pages"][1]["turns"][0]["items"][0]
        .as_object_mut()
        .ok_or("reviewer spawn")?
        .remove("receiver_agents");
    let error = rejected(native_history::normalize_native_history(&missing));
    assert!(
        error.contains("reviewer invocation must preserve exactly one receiver agent"),
        "{error}"
    );
    Ok(())
}

#[test]
fn host_session_provenance_admits_raw_spawn_without_receiver_agents() -> TestResult {
    let request = fixtures::host_provenance_request();
    let receipt = native_history::normalize_native_history(&request)?;
    assert_eq!(
        receipt["owner"]["invocation"]["receiver_role"],
        "codexy-sentinel"
    );
    assert_eq!(
        receipt["source"]["reviewer"]["capture"]["provenance"],
        request["reviewer"]["capture"]["provenance"]
    );
    Ok(())
}

#[test]
fn host_session_provenance_rejects_role_alias_drift() -> TestResult {
    let mut request = fixtures::host_provenance_request();
    request["reviewer"]["capture"]["provenance"]["session"]["source"]["subagent"]["thread_spawn"]
        ["agent_role"] = json!("codexy-cartographer");
    let error = rejected(native_history::normalize_native_history(&request));
    assert!(
        error.contains("capture reviewer agent role is contradictory"),
        "{error}"
    );
    Ok(())
}

#[test]
fn host_session_provenance_rejects_spawn_binding_drift() -> TestResult {
    let mut request = fixtures::host_provenance_request();
    request["reviewer"]["capture"]["provenance"]["spawn"]["id"] = json!("other-spawn");
    let error = rejected(native_history::normalize_native_history(&request));
    assert!(
        error.contains("capture spawn id does not match selected invocation"),
        "{error}"
    );
    Ok(())
}

#[test]
fn host_session_provenance_rejects_legacy_role_conflict() -> TestResult {
    let mut request = fixtures::host_provenance_request();
    request["owner"]["pages"][1]["turns"][0]["items"][0]["receiver_agents"] = json!([{
        "thread_id": "reviewer-thread",
        "agent_role": "codexy-cartographer"
    }]);
    let error = rejected(native_history::normalize_native_history(&request));
    assert!(
        error.contains("role conflicts with capture provenance"),
        "{error}"
    );
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
fn read_thread_capture_binds_requests_to_raw_pages() -> TestResult {
    let request = fixtures::read_thread_request();
    let receipt = native_history::normalize_native_history(&request)?;
    assert_eq!(
        receipt["source"]["owner"]["pages"],
        request["owner"]["pages"]
    );
    assert_eq!(
        receipt["source"]["reviewer"]["pages"],
        request["reviewer"]["pages"]
    );
    assert_eq!(
        receipt["source"]["owner"]["capture"],
        request["owner"]["capture"]
    );
    assert_eq!(
        receipt["source"]["reviewer"]["capture"],
        request["reviewer"]["capture"]
    );
    assert!(
        receipt["source"]["owner"]["pages"][1]["page"]
            .get("cursor")
            .is_none()
    );
    Ok(())
}

#[test]
fn read_thread_capture_rejects_cursor_drift_and_missing_requests() -> TestResult {
    let mut wrong = fixtures::read_thread_request();
    wrong["owner"]["capture"]["requests"][1]["cursor"] = json!("wrong-cursor");
    let error = rejected(native_history::normalize_native_history(&wrong));
    assert!(error.contains("capture request cursor"), "{error}");

    let mut missing = fixtures::read_thread_request();
    missing["reviewer"]["capture"]["requests"]
        .as_array_mut()
        .ok_or("capture requests")?
        .pop();
    let error = rejected(native_history::normalize_native_history(&missing));
    assert!(
        error.contains("capture requests must match pages"),
        "{error}"
    );

    let mut contradictory = fixtures::read_thread_request();
    contradictory["reviewer"]["pages"][1]["page"]["cursor"] = json!("wrong-cursor");
    let error = rejected(native_history::normalize_native_history(&contradictory));
    assert!(
        error.contains("page cursor disagrees with capture request"),
        "{error}"
    );
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
