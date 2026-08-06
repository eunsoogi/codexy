use serde_json::Value;

use super::schema::{
    array, check_dependencies, component_selection, exact_array, exact_map_string, exact_string,
};

pub(super) fn check(fixtures: &Value) -> Result<(), String> {
    exact_string(
        fixtures,
        "schema",
        "getcodexy.component-installation-cases.v1",
    )?;
    check_fixture_examples(fixtures)?;
    check_state_transitions(fixtures)
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

    let rollback = case(cases, "rollback-after-operation-failure")?;
    exact_string(rollback, "command", "install")?;
    exact_operands(rollback, &["devtools"])?;
    same_selection(rollback)?;
    exact_string(rollback, "outcome", "rolled-back")?;
    let receipt = object(rollback, "stdout")?;
    exact_map_string(receipt, "schema", "getcodexy.operation-receipt.v1")?;
    if receipt
        .get("operation_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err("rollback receipt must contain operation_id".to_owned());
    }

    let status = object(case(cases, "status-json")?, "stdout")?;
    exact_string(case(cases, "status-json")?, "command", "status")?;
    exact_operands(case(cases, "status-json")?, &[])?;
    exact_map_string(status, "schema", "getcodexy.status.v1")?;
    Ok(())
}

fn check_state_transitions(fixtures: &Value) -> Result<(), String> {
    let transitions = array(fixtures.get("state_transitions"), "state_transitions")?;
    for transition in transitions {
        let before = component_selection(transition.get("selection_before"), "selection_before")?;
        let after = component_selection(transition.get("selection_after"), "selection_after")?;
        exact_string(
            transition,
            "source_of_truth",
            "installed-component-inventory",
        )?;
        match transition.get("outcome").and_then(Value::as_str) {
            Some("completed") => check_dependencies(&after)?,
            Some("rejected") | Some("rolled-back") if before == after => {}
            _ => return Err("every transition must complete with closure or preserve state when rejected/rolled-back".to_owned()),
        }
    }
    transition(
        transitions,
        "install-all",
        "install",
        &[],
        &["core", "github", "devtools"],
        "completed",
    )?;
    transition(
        transitions,
        "install-is-additive",
        "install github",
        &["core", "devtools"],
        &["core", "github", "devtools"],
        "completed",
    )?;
    transition(
        transitions,
        "update-preserves",
        "update",
        &["core", "devtools"],
        &["core", "devtools"],
        "completed",
    )?;
    transition(
        transitions,
        "update-explicit-preserves",
        "update github",
        &["core", "github", "devtools"],
        &["core", "github", "devtools"],
        "completed",
    )?;
    transition(
        transitions,
        "reject-remove-core",
        "remove core",
        &["core", "github"],
        &["core", "github"],
        "rejected",
    )?;
    transition(
        transitions,
        "rollback-failed-install",
        "install devtools",
        &["core", "github"],
        &["core", "github"],
        "rolled-back",
    )?;
    transition(
        transitions,
        "bootstrap-default",
        "bootstrap",
        &[],
        &["core", "github", "devtools"],
        "completed",
    )
}

fn transition<'a>(
    transitions: &'a [Value],
    id: &str,
    command: &str,
    before: &[&str],
    after: &[&str],
    outcome: &str,
) -> Result<(), String> {
    let transition = case(transitions, id)?;
    exact_string(transition, "command", command)?;
    exact_selection(transition, "selection_before", before)?;
    exact_selection(transition, "selection_after", after)?;
    exact_string(transition, "outcome", outcome)
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

fn object<'a>(value: &'a Value, field: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{field} must be an object"))
}

#[cfg(test)]
#[path = "getcodexy_component_contract_cases_tests.rs"]
mod tests;
