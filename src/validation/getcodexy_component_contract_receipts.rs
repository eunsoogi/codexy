use serde_json::Value;

use super::super::schema::{array, exact_array_value, exact_map_string};

pub(super) fn rollback(receipt: &serde_json::Map<String, Value>) -> Result<(), String> {
    if receipt.len() != 11 {
        return Err("rollback receipt must have the complete mutation receipt shape".to_owned());
    }
    exact_map_string(receipt, "schema", "getcodexy.operation-receipt.v1")?;
    exact_map_string(receipt, "command", "install")?;
    exact_map_string(receipt, "outcome", "rolled-back")?;
    exact_array_value(
        receipt.get("requested_components"),
        &["devtools"],
        "requested_components",
    )?;
    exact_array_value(
        receipt.get("resolved_components"),
        &["core", "devtools"],
        "resolved_components",
    )?;
    for field in [
        "selection_before",
        "selection_after",
        "installed_components",
    ] {
        exact_array_value(receipt.get(field), &["core", "github"], field)?;
    }
    exact_map_string(receipt, "source_of_truth", "installed-component-inventory")?;
    let errors = array(receipt.get("errors"), "errors")?;
    if errors.len() != 1 {
        return Err("mutation receipt must contain exactly one error".to_owned());
    }
    exact_map_string(
        errors[0]
            .as_object()
            .ok_or_else(|| "mutation receipt error must be an object".to_owned())?,
        "code",
        "operation-failed",
    )
}

pub(super) fn status(receipt: &serde_json::Map<String, Value>) -> Result<(), String> {
    if receipt.len() != 7 {
        return Err("status receipt must have the complete status shape".to_owned());
    }
    exact_map_string(receipt, "schema", "getcodexy.status.v1")?;
    exact_map_string(receipt, "command", "status")?;
    exact_map_string(receipt, "outcome", "completed")?;
    for field in ["selected_components", "installed_components"] {
        exact_array_value(receipt.get(field), &["core", "github"], field)?;
    }
    exact_map_string(receipt, "source_of_truth", "installed-component-inventory")?;
    if array(receipt.get("errors"), "errors")?.is_empty() {
        Ok(())
    } else {
        Err("status receipt errors must be empty".to_owned())
    }
}
