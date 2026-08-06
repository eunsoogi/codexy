use serde_json::Value;

use super::schema::{
    array, component_selection, exact_array, exact_array_value, exact_map_string, exact_string,
    object,
};

#[path = "getcodexy_component_contract_receipts.rs"]
mod receipts;
#[path = "getcodexy_component_contract_transitions.rs"]
mod transitions;

pub(super) fn check(fixtures: &Value) -> Result<(), String> {
    exact_string(
        fixtures,
        "schema",
        "getcodexy.component-installation-cases.v1",
    )?;
    check_fixture_examples(fixtures)?;
    transitions::check(fixtures)
}

fn check_fixture_examples(fixtures: &Value) -> Result<(), String> {
    let cases = array(fixtures.get("fixtures"), "fixtures")?;
    let install_default = case(cases, "install-default")?;
    exact_string(install_default, "command", "install")?;
    exact_operands(install_default, &[])?;
    exact_selection(install_default, "selection_before", &[])?;
    exact_selection(
        install_default,
        "selection_after",
        &["core", "github", "devtools"],
    )?;
    exact_string(install_default, "outcome", "completed")?;

    let install_github = case(cases, "install-github")?;
    exact_string(install_github, "command", "install")?;
    exact_operands(install_github, &["github"])?;
    exact_selection(install_github, "selection_after", &["core", "github"])?;

    let update = case(cases, "update-preserves-selection")?;
    exact_string(update, "command", "update")?;
    exact_operands(update, &[])?;
    same_selection(update)?;
    exact_string(update, "outcome", "completed")?;

    rejected_case(
        cases,
        "remove-core-with-dependent",
        "remove",
        &["core"],
        "dependency-protected-removal",
    )?;
    rejected_case(
        cases,
        "remove-missing-target",
        "remove",
        &[],
        "missing-removal-target",
    )?;
    rejected_case(
        cases,
        "bootstrap-components-not-accepted",
        "bootstrap",
        &["core"],
        "components-not-accepted",
    )?;
    rejected_case(
        cases,
        "update-no-recorded-selection",
        "update",
        &[],
        "no-recorded-selection",
    )?;
    rejected_case(
        cases,
        "update-inconsistent-installed-state",
        "update",
        &[],
        "inconsistent-installed-state",
    )?;
    rejected_case(
        cases,
        "install-unknown-component",
        "install",
        &["unknown"],
        "unknown-component",
    )?;
    check_update_inventory_states(cases)?;

    let rollback = case(cases, "rollback-after-operation-failure")?;
    exact_string(rollback, "command", "install")?;
    exact_operands(rollback, &["devtools"])?;
    same_selection(rollback)?;
    exact_string(rollback, "outcome", "rolled-back")?;
    let outer_error = object(rollback, "error")?;
    if outer_error.len() != 3 {
        return Err("rollback error must have the complete outer error shape".to_owned());
    }
    exact_map_string(outer_error, "code", "operation-failed")?;
    exact_map_string(outer_error, "message", "install failed")?;
    exact_array_value(
        outer_error.get("components"),
        &["core", "github", "devtools"],
        "error.components",
    )?;
    let receipt = object(rollback, "stdout")?;
    receipts::rollback(receipt)?;
    if receipt
        .get("operation_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err("rollback receipt must contain operation_id".to_owned());
    }

    receipts::statuses(cases)?;
    receipts::doctor(cases)?;
    Ok(())
}

fn check_update_inventory_states(cases: &[Value]) -> Result<(), String> {
    let absent = case(cases, "update-no-recorded-selection")?;
    inventory(
        absent,
        "inventory_before",
        "selection_before",
        "absent",
        None,
    )?;
    inventory(absent, "inventory_after", "selection_after", "absent", None)?;
    let empty = case(cases, "update-present-empty-inventory")?;
    inventory(
        empty,
        "inventory_before",
        "selection_before",
        "present",
        Some(&[]),
    )?;
    inventory(
        empty,
        "inventory_after",
        "selection_after",
        "present",
        Some(&[]),
    )?;
    same_selection(empty)?;
    exact_string(empty, "outcome", "completed")?;
    let invalid = case(cases, "update-inconsistent-installed-state")?;
    inventory(
        invalid,
        "inventory_before",
        "selection_before",
        "present",
        Some(&["github"]),
    )?;
    inventory(
        invalid,
        "inventory_after",
        "selection_after",
        "present",
        Some(&["github"]),
    )
}

fn inventory(
    value: &Value,
    field: &str,
    selection: &str,
    state: &str,
    components: Option<&[&str]>,
) -> Result<(), String> {
    let inventory = object(value, field)?;
    exact_map_string(inventory, "state", state)?;
    match components {
        Some(components) => {
            exact_array_value(inventory.get("components"), components, field)?;
            exact_array_value(value.get(selection), components, selection)
        }
        None if inventory.contains_key("components") => {
            Err(format!("{field} absent inventory must omit components"))
        }
        None => exact_array_value(value.get(selection), &[], selection),
    }
}

fn rejected_case(
    cases: &[Value],
    id: &str,
    command: &str,
    operands: &[&str],
    code: &str,
) -> Result<(), String> {
    let fixture = case(cases, id)?;
    exact_string(fixture, "command", command)?;
    exact_operands(fixture, operands)?;
    same_selection(fixture)?;
    exact_string(fixture, "outcome", "rejected")?;
    exact_map_string(object(fixture, "error")?, "code", code)
}

fn exact_operands(fixture: &Value, expected: &[&str]) -> Result<(), String> {
    exact_array(fixture, "requested_components", expected)
}

fn case<'a>(cases: &'a [Value], id: &str) -> Result<&'a Value, String> {
    cases
        .iter()
        .find(|case| case.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("fixture {id} is required"))
}

fn same_selection(fixture: &Value) -> Result<(), String> {
    let before = component_selection(fixture.get("selection_before"), "selection_before")?;
    let after = component_selection(fixture.get("selection_after"), "selection_after")?;
    if before == after {
        Ok(())
    } else {
        Err("rejected or rolled-back fixture must preserve selection".to_owned())
    }
}

fn exact_selection(fixture: &Value, field: &str, expected: &[&str]) -> Result<(), String> {
    let actual = component_selection(fixture.get(field), field)?;
    if actual.iter().map(String::as_str).collect::<Vec<_>>() == expected {
        Ok(())
    } else {
        Err(format!("{field} must be {expected:?}"))
    }
}

#[cfg(test)]
#[path = "getcodexy_component_contract_cases_tests.rs"]
mod tests;
