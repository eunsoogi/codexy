use serde_json::Value;

use super::super::schema::{array, exact_array_value, exact_map_string, object};

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

pub(super) fn statuses(cases: &[Value]) -> Result<(), String> {
    for (id, state, components, consistency, error) in [
        (
            "status-json",
            "present",
            &["core", "github"][..],
            "consistent",
            None,
        ),
        (
            "status-absent-json",
            "absent",
            &[][..],
            "not-recorded",
            None,
        ),
        (
            "status-present-empty-json",
            "present",
            &[][..],
            "consistent",
            None,
        ),
        (
            "status-inconsistent-json",
            "present",
            &["github"][..],
            "inconsistent",
            Some("inconsistent-installed-state"),
        ),
    ] {
        let fixture = case(cases, id)?;
        exact_map_string(object(fixture, "stdout")?, "command", "status")?;
        exact_array_value(
            fixture.get("requested_components"),
            &[],
            "requested_components",
        )?;
        for field in ["selection_before", "selection_after"] {
            exact_array_value(fixture.get(field), components, field)?;
        }
        status(
            object(fixture, "stdout")?,
            state,
            components,
            consistency,
            error,
        )?;
    }
    Ok(())
}

pub(super) fn doctor(cases: &[Value]) -> Result<(), String> {
    let fixture = case(cases, "doctor-json")?;
    exact_map_string(object(fixture, "stdout")?, "command", "doctor")?;
    exact_array_value(
        fixture.get("requested_components"),
        &[],
        "requested_components",
    )?;
    for field in ["selection_before", "selection_after"] {
        exact_array_value(fixture.get(field), &["core", "github"], field)?;
    }
    let receipt = object(fixture, "stdout")?;
    if receipt.len() != 9 {
        return Err("doctor receipt must have the complete doctor shape".to_owned());
    }
    exact_map_string(receipt, "schema", "getcodexy.doctor.v1")?;
    exact_map_string(receipt, "command", "doctor")?;
    exact_map_string(receipt, "outcome", "completed")?;
    inventory(receipt, "present", &["core", "github"])?;
    exact_map_string(receipt, "inventory_consistency", "consistent")?;
    let readiness = object_value(receipt, "host_readiness")?;
    if readiness.len() != 2 {
        return Err("doctor host_readiness must have the complete shape".to_owned());
    }
    exact_map_string(readiness, "state", "ready")?;
    exact_array_value(
        readiness.get("missing_requirements"),
        &[],
        "missing_requirements",
    )?;
    let health = array(receipt.get("component_health"), "component_health")?;
    if health.len() != 2 {
        return Err("doctor component_health must report every installed component".to_owned());
    }
    for (entry, component) in health.iter().zip(["core", "github"]) {
        let entry = entry
            .as_object()
            .ok_or_else(|| "doctor component_health entry must be an object".to_owned())?;
        if entry.len() != 2 {
            return Err("doctor component_health entry must have the complete shape".to_owned());
        }
        exact_map_string(entry, "component", component)?;
        exact_map_string(entry, "state", "healthy")?;
    }
    exact_map_string(receipt, "source_of_truth", "installed-component-inventory")?;
    empty_errors(receipt)
}

fn status(
    receipt: &serde_json::Map<String, Value>,
    state: &str,
    components: &[&str],
    consistency: &str,
    error: Option<&str>,
) -> Result<(), String> {
    if receipt.len() != 9 {
        return Err("status receipt must have the complete status shape".to_owned());
    }
    exact_map_string(receipt, "schema", "getcodexy.status.v1")?;
    exact_map_string(receipt, "command", "status")?;
    exact_map_string(receipt, "outcome", "completed")?;
    inventory(receipt, state, components)?;
    exact_map_string(receipt, "inventory_consistency", consistency)?;
    for field in ["selected_components", "installed_components"] {
        exact_array_value(receipt.get(field), components, field)?;
    }
    exact_map_string(receipt, "source_of_truth", "installed-component-inventory")?;
    match error {
        Some(code) => one_error(receipt, code),
        None => empty_errors(receipt),
    }
}

fn inventory(
    receipt: &serde_json::Map<String, Value>,
    state: &str,
    components: &[&str],
) -> Result<(), String> {
    let inventory = object_value(receipt, "inventory")?;
    exact_map_string(inventory, "state", state)?;
    if state == "absent" {
        if inventory.len() == 1 {
            Ok(())
        } else {
            Err("absent receipt inventory must omit components".to_owned())
        }
    } else if inventory.len() == 2 {
        exact_array_value(
            inventory.get("components"),
            components,
            "inventory.components",
        )
    } else {
        Err("present receipt inventory must have state and components".to_owned())
    }
}

fn empty_errors(receipt: &serde_json::Map<String, Value>) -> Result<(), String> {
    if array(receipt.get("errors"), "errors")?.is_empty() {
        Ok(())
    } else {
        Err("read receipt errors must be empty".to_owned())
    }
}

fn one_error(receipt: &serde_json::Map<String, Value>, code: &str) -> Result<(), String> {
    let errors = array(receipt.get("errors"), "errors")?;
    if errors.len() != 1 {
        return Err("read receipt must contain exactly one error".to_owned());
    }
    let error = errors[0]
        .as_object()
        .ok_or_else(|| "read receipt error must be an object".to_owned())?;
    if error.len() != 1 {
        return Err("read receipt error must have the complete shape".to_owned());
    }
    exact_map_string(error, "code", code)
}

fn case<'a>(cases: &'a [Value], id: &str) -> Result<&'a Value, String> {
    cases
        .iter()
        .find(|case| case.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("fixture {id} is required"))
}

fn object_value<'a>(
    value: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{field} must be an object"))
}
