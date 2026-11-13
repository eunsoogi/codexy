use super::support::{TestResult, assert_rejected, classification_request, remove_trigger};
use serde_json::json;

#[test]
fn incomplete_duplicate_unknown_and_malformed_risk_evidence_is_rejected() -> TestResult {
    let mut missing = classification_request("middle", false, &[], &[], None);
    remove_trigger(&mut missing, "materially_shared_executable_contract");
    assert_rejected(&missing, "every material-risk trigger");

    let mut duplicate = classification_request("middle", false, &[], &[], None);
    duplicate["risk_assessment"]["risk_decisions"]
        .as_array_mut()
        .expect("trigger array")
        .push(json!({"kind":"security", "applies":true}));
    assert_rejected(&duplicate, "duplicates a trigger");

    let mut unknown = classification_request("middle", false, &[], &[], None);
    unknown["risk_assessment"]["risk_decisions"]
        .as_array_mut()
        .expect("trigger array")
        .push(json!({"kind":"unknown_risk", "applies":false}));
    assert_rejected(&unknown, "unknown trigger");

    let mut malformed = classification_request("middle", false, &[], &[], None);
    malformed["risk_assessment"]["risk_decisions"][0]["applies"] = json!("yes");
    assert_rejected(&malformed, "invalid type");

    let mut duplicate_legacy =
        classification_request("middle", false, &[], &[("durable_delegation", true)], None);
    duplicate_legacy["risk_assessment"]["risk_decisions"]
        .as_array_mut()
        .expect("trigger array")
        .push(json!({"kind":"durable_delegation", "applies":false}));
    assert_rejected(&duplicate_legacy, "duplicates a trigger");

    let mut missing_selection = classification_request("middle", false, &[], &[], None);
    missing_selection
        .as_object_mut()
        .unwrap()
        .remove("review_requested");
    assert_rejected(&missing_selection, "review_requested");
    let prior = classification_request("middle", false, &[], &[], Some("strict"));
    assert_rejected(&prior, "unknown field");
    Ok(())
}

#[test]
fn v2_payload_without_material_risk_decisions_is_not_treated_as_low_risk() -> TestResult {
    let mut legacy = classification_request(
        "low_risk",
        true,
        &[],
        &[("durable_delegation", true), ("multi_lane_ownership", true)],
        None,
    );
    legacy["risk_assessment"]["schema"] = json!("codexy.workflow-profile-classification.v2");
    let triggers = legacy["risk_assessment"]["risk_decisions"]
        .as_array_mut()
        .expect("trigger array");
    triggers.retain(|item| {
        !matches!(
            item["kind"].as_str(),
            Some("materially_shared_executable_contract" | "explicit_review_requirement")
        )
    });
    assert_rejected(&legacy, "unsupported schema");
    Ok(())
}

#[test]
fn explicit_review_obligation_cannot_be_skipped() {
    let mut request =
        classification_request("middle", false, &["explicit_review_requirement"], &[], None);
    request["review_requested"] = json!(false);
    assert_rejected(&request, "explicit review requirement");
}
