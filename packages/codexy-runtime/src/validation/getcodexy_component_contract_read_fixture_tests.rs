use serde_json::json;

use super::check;

fn fixtures() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../packages/getcodexy/tests/fixtures/component-installation-cases.json"
    ))
    .expect("fixture JSON")
}

fn fixture_mut<'a>(fixtures: &'a mut serde_json::Value, id: &str) -> &'a mut serde_json::Value {
    fixtures["fixtures"]
        .as_array_mut()
        .expect("fixture cases")
        .iter_mut()
        .find(|fixture| fixture["id"] == id)
        .expect("named fixture")
}

#[test]
fn rejects_shared_read_fixture_envelope_drift() {
    let mut status_command = fixtures();
    fixture_mut(&mut status_command, "status-json")["command"] = json!("install");
    assert!(check(&status_command).is_err());

    let mut doctor_outcome = fixtures();
    fixture_mut(&mut doctor_outcome, "doctor-json")["outcome"] = json!("rejected");
    assert!(check(&doctor_outcome).is_err());

    let mut status_operands = fixtures();
    fixture_mut(&mut status_operands, "status-json")["requested_components"] = json!(["core"]);
    assert!(check(&status_operands).is_err());

    let mut doctor_selection = fixtures();
    fixture_mut(&mut doctor_selection, "doctor-json")["selection_after"] = json!(["core"]);
    assert!(check(&doctor_selection).is_err());
}
