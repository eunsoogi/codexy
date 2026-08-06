use std::path::Path;

use serde_json::Value;

#[path = "getcodexy_component_contract_cases.rs"]
mod cases;
#[path = "getcodexy_component_contract_schema.rs"]
mod schema;

use schema::{
    COMPONENTS, array, exact_array, exact_array_value, exact_map_string, exact_string, object,
    object_value,
};

pub(super) fn check(_: &Path) -> Vec<String> {
    match contract_and_fixtures() {
        Ok(()) => Vec::new(),
        Err(error) => vec![error],
    }
}

fn contract_and_fixtures() -> Result<(), String> {
    let root = crate::paths::repo_root().map_err(|error| error.to_string())?;
    let contract =
        load(&root.join("packages/getcodexy/contracts/component-installation-contract.json"))?;
    let fixtures =
        load(&root.join("packages/getcodexy/tests/fixtures/component-installation-cases.json"))?;
    check_contract(&contract)?;
    cases::check(&fixtures)?;
    let documentation =
        std::fs::read_to_string(root.join("docs/getcodexy-component-installation.md"))
            .map_err(|error| format!("reading component-installation documentation: {error}"))?;
    for required in [
        "target public contract for the 1.4.0",
        "There is deliberately no `getcodexy rollback RECEIPT_ID` command",
        "packages/getcodexy/contracts/component-installation-contract.json",
    ] {
        if !documentation.contains(required) {
            return Err(format!(
                "component-installation documentation must include {required:?}"
            ));
        }
    }
    Ok(())
}

fn load(path: &Path) -> Result<Value, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("{}: {error}", crate::paths::display_relative(path)))
        .and_then(|text| {
            serde_json::from_str(&text)
                .map_err(|error| format!("{}: {error}", crate::paths::display_relative(path)))
        })
}

fn check_contract(contract: &Value) -> Result<(), String> {
    exact_string(
        contract,
        "schema",
        "getcodexy.component-installation-contract.v1",
    )?;
    exact_array(contract, "components", COMPONENTS)?;
    let products = object(contract, "component_products")?;
    for (component, product) in [
        ("core", "codexy"),
        ("github", "codexy-github"),
        ("devtools", "codexy-devtools"),
    ] {
        if products.get(component).and_then(Value::as_str) != Some(product) {
            return Err(format!("component_products.{component} must be {product}"));
        }
    }
    let dependencies = object(contract, "dependencies")?;
    for (component, expected) in [
        ("core", &[][..]),
        ("github", &["core"][..]),
        ("devtools", &["core"][..]),
    ] {
        exact_array_value(
            dependencies.get(component),
            expected,
            &format!("dependencies.{component}"),
        )?;
    }
    exact_map_string(
        object(contract, "source_of_truth")?,
        "kind",
        "installed-component-inventory",
    )?;
    let commands = object(contract, "commands")?;
    exact_map_string(object_value(commands, "install")?, "no_arguments", "all")?;
    exact_map_string(
        object_value(commands, "install")?,
        "selection",
        "add-resolved-request-to-installed-selection",
    )?;
    exact_map_string(
        object_value(commands, "update")?,
        "selection",
        "preserve-installed",
    )?;
    if object_value(commands, "remove")?.get("requires_components") != Some(&Value::Bool(true)) {
        return Err("remove.requires_components must be true".to_owned());
    }
    exact_map_string(
        object_value(commands, "rollback")?,
        "kind",
        "automatic-mutation-failure-recovery",
    )?;
    exact_map_string(
        object_value(commands, "rollback")?,
        "manual_command",
        "deferred-to-issue-557",
    )?;
    let output = object(contract, "machine_readable_output")?;
    exact_map_string(output, "flag", "--json")?;
    exact_map_string(
        output,
        "mutation_receipt_schema",
        "getcodexy.operation-receipt.v1",
    )?;
    exact_map_string(output, "status_schema", "getcodexy.status.v1")?;
    let fields = array(
        output.get("required_mutation_receipt_fields"),
        "required_mutation_receipt_fields",
    )?;
    if !fields
        .iter()
        .any(|field| field.as_str() == Some("operation_id"))
    {
        return Err("mutation receipts must require operation_id".to_owned());
    }
    Ok(())
}
