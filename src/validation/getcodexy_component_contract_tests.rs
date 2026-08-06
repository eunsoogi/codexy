use serde_json::json;

use super::check_contract;

fn contract() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../packages/getcodexy/contracts/component-installation-contract.json"
    ))
    .expect("contract JSON")
}

#[test]
fn rejects_a_missing_json_flag_for_each_public_command() {
    for command in [
        "install",
        "update",
        "remove",
        "status",
        "doctor",
        "bootstrap",
    ] {
        let mut contract = contract();
        contract["commands"][command]["usage"] = json!(command);

        assert!(check_contract(&contract).is_err(), "{command}");
    }
}
