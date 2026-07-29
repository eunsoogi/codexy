use std::{fs, path::Path};

use serde_json::Value;

use crate::paths::display_relative;

pub(super) const ID: &str = "codexy-main-squash";
const PATH: &str = "skills/git-workflow/references/merge-authorization-contract.json";

pub(super) fn check(plugin_root: &Path, record: &Value, errors: &mut Vec<String>) {
    let path = plugin_root.join(PATH);
    let contract = match fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    {
        Some(contract) => contract,
        None => {
            errors.push(format!(
                "{} must define the checked merge-authorization contract",
                display_relative(&path)
            ));
            return;
        }
    };
    require(&contract, "contractId", ID, errors);
    if contract.get("contractVersion").and_then(Value::as_u64) != Some(1) {
        errors.push("merge authorization contractVersion must be 1".into());
    }
    require(&contract, "mergeClass", "squash", errors);
    require(&contract, "target", "current-pull-request", errors);
    require(&contract, "recordIssuer", "maintainer-recorded", errors);
    require(record, "contractId", ID, errors);
    require(record, "target", "current-pull-request", errors);
    if record.get("contractVersion").and_then(Value::as_u64) != Some(1) {
        errors.push("merge authorization contractVersion must be 1".into());
    }
    require(record, "recordIssuer", "maintainer-recorded", errors);
}

fn require(value: &Value, field: &str, expected: &str, errors: &mut Vec<String>) {
    if value.get(field).and_then(Value::as_str) != Some(expected) {
        errors.push(format!("merge authorization {field} must be {expected:?}"));
    }
}
