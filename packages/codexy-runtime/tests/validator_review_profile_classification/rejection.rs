use super::support::{TestResult, assert_rejected, classification_request, remove_trigger};
use serde_json::json;

#[test]
fn incomplete_duplicate_unknown_and_malformed_risk_evidence_is_rejected() -> TestResult {
    let mut missing = classification_request("middle", false, &[], &[], None);
    remove_trigger(&mut missing, "materially_shared_executable_contract");
    assert_rejected(&missing, "every material-risk trigger");

    let mut duplicate = classification_request("middle", false, &[], &[], None);
    duplicate["classification"]["strict_triggers"]
        .as_array_mut()
        .expect("trigger array")
        .push(json!({"kind":"security", "applies":true}));
    assert_rejected(&duplicate, "duplicates a trigger");

    let mut unknown = classification_request("middle", false, &[], &[], None);
    unknown["classification"]["strict_triggers"]
        .as_array_mut()
        .expect("trigger array")
        .push(json!({"kind":"unknown_risk", "applies":false}));
    assert_rejected(&unknown, "unknown trigger");

    let mut malformed = classification_request("middle", false, &[], &[], None);
    malformed["classification"]["strict_triggers"][0]["applies"] = json!("yes");
    assert_rejected(&malformed, "invalid type");

    let mut duplicate_legacy = classification_request(
        "middle",
        false,
        &[],
        &[("durable_delegation", true)],
        None,
    );
    duplicate_legacy["classification"]["strict_triggers"]
        .as_array_mut()
        .expect("trigger array")
        .push(json!({"kind":"durable_delegation", "applies":false}));
    assert_rejected(&duplicate_legacy, "duplicates a trigger");

    let contradictory = classification_request("middle", true, &[], &[], None);
    assert_rejected(&contradictory, "no eligible non-strict review route");
    let unknown_class = classification_request("unknown", false, &[], &[], None);
    assert_rejected(&unknown_class, "no eligible non-strict review route");
    Ok(())
}

#[test]
fn v2_payload_without_material_risk_decisions_is_not_treated_as_low_risk() -> TestResult {
    let mut legacy = classification_request(
        "low_risk",
        true,
        &[],
        &[
            ("durable_delegation", true),
            ("multi_lane_ownership", true),
        ],
        None,
    );
    legacy["classification"]["schema"] = json!("codexy.workflow-profile-classification.v2");
    let triggers = legacy["classification"]["strict_triggers"]
        .as_array_mut()
        .expect("trigger array");
    triggers.retain(|item| {
        !matches!(
            item["kind"].as_str(),
            Some("materially_shared_executable_contract" | "explicit_strict_requirement")
        )
    });
    assert_rejected(&legacy, "unsupported schema");
    Ok(())
}
