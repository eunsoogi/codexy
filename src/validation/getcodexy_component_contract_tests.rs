use serde_json::json;

use super::{check_contract, validate_contract_root};

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

#[test]
fn rejects_receipt_field_declaration_drift() {
    for field in ["required_mutation_receipt_fields", "required_status_fields"] {
        let mut contract = contract();
        contract["machine_readable_output"][field] = json!(["schema"]);

        assert!(check_contract(&contract).is_err(), "{field}");
    }
}

#[test]
fn source_contract_root_fails_closed_for_each_missing_artifact() {
    for missing in ["documentation", "contract", "fixtures"] {
        let root = tempfile::tempdir().expect("source root");
        let docs = root.path().join("docs");
        let package = root.path().join("packages/getcodexy");
        std::fs::create_dir_all(&docs).expect("docs");
        std::fs::create_dir_all(package.join("contracts")).expect("contracts");
        std::fs::create_dir_all(package.join("tests/fixtures")).expect("fixtures");
        if missing != "documentation" {
            std::fs::write(docs.join("getcodexy-component-installation.md"), "target public contract for the 1.4.0\nThere is deliberately no `getcodexy rollback RECEIPT_ID` command\npackages/getcodexy/contracts/component-installation-contract.json\n").expect("docs");
        }
        if missing != "contract" {
            std::fs::write(
                package.join("contracts/component-installation-contract.json"),
                include_str!(
                    "../../packages/getcodexy/contracts/component-installation-contract.json"
                ),
            )
            .expect("contract");
        }
        if missing != "fixtures" {
            std::fs::write(
                package.join("tests/fixtures/component-installation-cases.json"),
                include_str!(
                    "../../packages/getcodexy/tests/fixtures/component-installation-cases.json"
                ),
            )
            .expect("fixtures");
        }
        assert!(validate_contract_root(root.path()).is_err(), "{missing}");
    }
}
