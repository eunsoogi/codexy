use serde_json::Value;

use super::super::schema::{check_dependencies, component_selection, exact_string};

#[derive(Debug, PartialEq)]
struct Command {
    name: String,
    operands: Vec<String>,
}

#[derive(Debug, PartialEq)]
struct Transition {
    command: Command,
    before: Vec<String>,
    after: Vec<String>,
    outcome: String,
    error: Option<String>,
}

struct Expected {
    id: &'static str,
    command: &'static str,
    operands: &'static [&'static str],
    before: &'static [&'static str],
    after: &'static [&'static str],
    outcome: &'static str,
    error: Option<&'static str>,
}

#[path = "getcodexy_component_contract_transition_table.rs"]
mod table;

use table::EXPECTED;

pub(super) fn check(fixtures: &Value) -> Result<(), String> {
    let transitions = fixtures
        .get("state_transitions")
        .and_then(Value::as_array)
        .ok_or_else(|| "state_transitions must be an array".to_owned())?;
    if transitions.len() != EXPECTED.len() {
        return Err("state_transitions must contain the exhaustive fourteen-row table".to_owned());
    }
    for expected in EXPECTED {
        let value = transitions
            .iter()
            .find(|row| row.get("id").and_then(Value::as_str) == Some(expected.id))
            .ok_or_else(|| format!("state transition {} is required", expected.id))?;
        let actual = parse(value)?;
        compare(expected, &actual)?;
    }
    Ok(())
}

fn parse(value: &Value) -> Result<Transition, String> {
    exact_string(value, "source_of_truth", "installed-component-inventory")?;
    let mut words = value
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| "transition command must be a string".to_owned())?
        .split_whitespace();
    let command = Command {
        name: words
            .next()
            .ok_or_else(|| "transition command must not be empty".to_owned())?
            .to_owned(),
        operands: words.map(ToOwned::to_owned).collect(),
    };
    let before = component_selection(value.get("selection_before"), "selection_before")?;
    let after = component_selection(value.get("selection_after"), "selection_after")?;
    let outcome = value
        .get("outcome")
        .and_then(Value::as_str)
        .ok_or_else(|| "transition outcome must be a string".to_owned())?
        .to_owned();
    let error = value
        .get("error")
        .map(|error| {
            error
                .get("code")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .ok_or_else(|| "transition error.code must be a string".to_owned())
        })
        .transpose()?;
    match outcome.as_str() {
        "completed" => check_dependencies(&after)?,
        "rejected" | "rolled-back" if before == after => {}
        _ => {
            return Err(
                "every transition must complete with closure or preserve inventory".to_owned(),
            );
        }
    }
    Ok(Transition {
        command,
        before,
        after,
        outcome,
        error,
    })
}

fn compare(expected: &Expected, actual: &Transition) -> Result<(), String> {
    let expected_operands = expected
        .operands
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    let expected_before = expected
        .before
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    let expected_after = expected
        .after
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    let expected_error = expected.error.map(ToOwned::to_owned);
    if actual.command.name == expected.command
        && actual.command.operands == expected_operands
        && actual.before == expected_before
        && actual.after == expected_after
        && actual.outcome == expected.outcome
        && actual.error == expected_error
    {
        Ok(())
    } else {
        Err(format!(
            "state transition {} does not match its typed contract row",
            expected.id
        ))
    }
}
