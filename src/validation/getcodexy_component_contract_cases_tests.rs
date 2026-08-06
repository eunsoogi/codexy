use serde_json::json;

use super::check;

fn fixtures() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../packages/getcodexy/tests/fixtures/component-installation-cases.json"
    ))
    .expect("fixture JSON")
}

#[test]
fn rejects_default_install_transition_drift() {
    let mut fixtures = fixtures();
    fixtures["state_transitions"][0]["selection_after"] = json!(["core"]);

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_update_selection_drift() {
    let mut fixtures = fixtures();
    fixtures["state_transitions"][6]["selection_after"] = json!(["core"]);

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_removal_error_drift() {
    let mut fixtures = fixtures();
    fixtures["fixtures"][3]["error"]["code"] = json!("unknown-component");

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_fixture_command_drift() {
    let mut fixtures = fixtures();
    fixtures["fixtures"][1]["command"] = json!("update");

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_transition_operand_drift() {
    let mut fixtures = fixtures();
    fixtures["state_transitions"][2]["command"] = json!("install devtools");

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_transition_error_drift() {
    let mut fixtures = fixtures();
    fixtures["state_transitions"][12]["error"]["code"] = json!("unknown-component");

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_absent_inventory_became_present_empty() {
    let mut fixtures = fixtures();
    let absent = fixtures["fixtures"]
        .as_array_mut()
        .expect("fixture cases")
        .iter_mut()
        .find(|fixture| fixture["id"] == "update-no-recorded-selection")
        .expect("absent update fixture");
    absent["inventory_before"] = json!({"state": "present", "components": []});

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_present_invalid_inventory_became_present_empty() {
    let mut fixtures = fixtures();
    let invalid = fixtures["fixtures"]
        .as_array_mut()
        .expect("fixture cases")
        .iter_mut()
        .find(|fixture| fixture["id"] == "update-inconsistent-installed-state")
        .expect("invalid update fixture");
    invalid["inventory_before"]["components"] = json!([]);

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_rollback_receipt_inventory_drift() {
    let mut fixtures = fixtures();
    fixtures["fixtures"][4]["stdout"]["installed_components"] = json!(["core"]);

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_rollback_receipt_error_drift() {
    let mut fixtures = fixtures();
    fixtures["fixtures"][4]["stdout"]["errors"][0]["code"] = json!("unknown-component");

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_status_receipt_cross_field_drift() {
    let mut fixtures = fixtures();
    fixtures["fixtures"][5]["stdout"]["installed_components"] = json!(["core"]);

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_rollback_outcome_drift() {
    let mut fixtures = fixtures();
    fixtures["fixtures"][4]["outcome"] = json!("rejected");

    assert!(check(&fixtures).is_err());
}

#[test]
fn rejects_status_schema_drift() {
    let mut fixtures = fixtures();
    fixtures["fixtures"][5]["stdout"]["schema"] = json!("getcodexy.operation-receipt.v1");

    assert!(check(&fixtures).is_err());
}
