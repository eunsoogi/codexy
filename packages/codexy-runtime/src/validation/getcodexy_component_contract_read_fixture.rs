use serde_json::Value;

use super::super::super::schema::{exact_array_value, exact_map_string, exact_string, object};

pub(super) struct ReadFixture<'a> {
    pub(super) receipt: &'a serde_json::Map<String, Value>,
}

pub(super) fn read<'a>(
    cases: &'a [Value],
    id: &str,
    command: &str,
    inventory_state: &str,
    selection: &[&str],
) -> Result<ReadFixture<'a>, String> {
    let fixture = case(cases, id)?;
    exact_string(fixture, "command", command)?;
    exact_string(fixture, "outcome", "completed")?;
    exact_array_value(
        fixture.get("requested_components"),
        &[],
        "requested_components",
    )?;
    for field in ["selection_before", "selection_after"] {
        exact_array_value(fixture.get(field), selection, field)?;
    }
    let receipt = object(fixture, "stdout")?;
    exact_map_string(receipt, "command", command)?;
    exact_map_string(receipt, "outcome", "completed")?;
    inventory(receipt, inventory_state, selection)?;
    Ok(ReadFixture { receipt })
}

fn inventory(
    receipt: &serde_json::Map<String, Value>,
    state: &str,
    selection: &[&str],
) -> Result<(), String> {
    let inventory = receipt
        .get("inventory")
        .and_then(Value::as_object)
        .ok_or_else(|| "inventory must be an object".to_owned())?;
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
            selection,
            "inventory.components",
        )
    } else {
        Err("present receipt inventory must have state and components".to_owned())
    }
}

fn case<'a>(cases: &'a [Value], id: &str) -> Result<&'a Value, String> {
    cases
        .iter()
        .find(|case| case.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("fixture {id} is required"))
}
