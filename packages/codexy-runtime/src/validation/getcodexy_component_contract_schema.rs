use serde_json::Value;

pub(super) const COMPONENTS: &[&str] = &["core", "github", "devtools"];
pub(super) const DOMAIN_ERRORS: &[&str] = &[
    "component-version-mismatch",
    "components-not-accepted",
    "conflicting-component-request",
    "conflicting-installed-state",
    "dependency-protected-removal",
    "hook-state-unavailable",
    "incompatible-component-selection",
    "inconsistent-installed-state",
    "installed-state-mismatch",
    "invalid-installed-inventory",
    "missing-removal-target",
    "mixed-version-state",
    "no-recorded-selection",
    "operation-failed",
    "required-hook-disabled",
    "required-hook-trust-missing",
    "required-hook-trust-stale",
    "unknown-component",
    "unknown-installed-component",
];

pub(super) fn component_selection(
    value: Option<&Value>,
    field: &str,
) -> Result<Vec<String>, String> {
    let values = array(value, field)?;
    let selection = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("{field} must contain strings"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected = COMPONENTS
        .iter()
        .filter(|component| selection.contains(&component.to_string()))
        .map(|component| (*component).to_owned())
        .collect::<Vec<_>>();
    if selection != expected {
        return Err(format!(
            "{field} must use canonical component order without duplicates"
        ));
    }
    Ok(selection)
}

pub(super) fn check_dependencies(selection: &[String]) -> Result<(), String> {
    if (selection.iter().any(|component| component == "github")
        || selection.iter().any(|component| component == "devtools"))
        && !selection.iter().any(|component| component == "core")
    {
        return Err("successful selection with github or devtools must include core".to_owned());
    }
    Ok(())
}

pub(super) fn object<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    object_value(
        value
            .as_object()
            .ok_or_else(|| "contract root must be an object".to_owned())?,
        field,
    )
}

pub(super) fn object_value<'a>(
    value: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{field} must be an object"))
}

pub(super) fn array<'a>(value: Option<&'a Value>, field: &str) -> Result<&'a Vec<Value>, String> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{field} must be an array"))
}

pub(super) fn exact_string(value: &Value, field: &str, expected: &str) -> Result<(), String> {
    if value.get(field).and_then(Value::as_str) != Some(expected) {
        return Err(format!("{field} must be {expected}"));
    }
    Ok(())
}

pub(super) fn exact_map_string(
    value: &serde_json::Map<String, Value>,
    field: &str,
    expected: &str,
) -> Result<(), String> {
    if value.get(field).and_then(Value::as_str) != Some(expected) {
        return Err(format!("{field} must be {expected}"));
    }
    Ok(())
}

pub(super) fn exact_array(value: &Value, field: &str, expected: &[&str]) -> Result<(), String> {
    exact_array_value(value.get(field), expected, field)
}

pub(super) fn exact_array_value(
    value: Option<&Value>,
    expected: &[&str],
    field: &str,
) -> Result<(), String> {
    let actual = array(value, field)?
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("{field} must contain strings"))?;
    if actual != expected {
        return Err(format!("{field} must be {expected:?}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{check_dependencies, component_selection};

    #[test]
    fn rejects_a_successful_dependent_selection_without_core() {
        let error = check_dependencies(&["github".to_owned()]).expect_err("must reject");

        assert!(error.contains("must include core"));
    }

    #[test]
    fn rejects_noncanonical_component_order() {
        let value = json!(["github", "core"]);
        let error = component_selection(Some(&value), "selection").expect_err("must reject");

        assert!(error.contains("canonical component order"));
    }
}
