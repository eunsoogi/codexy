use std::path::Path;

use serde_json::Value;

#[path = "getcodexy_component_contract_cases.rs"]
mod cases;
#[path = "getcodexy_component_manifest.rs"]
mod component_manifest;
#[path = "getcodexy_component_contract_schema.rs"]
mod schema;

use schema::{
    COMPONENTS, exact_array, exact_array_value, exact_map_string, exact_string, object,
    object_value,
};

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    match contract_and_fixtures(plugin_root) {
        Ok(()) => Vec::new(),
        Err(error) => vec![error],
    }
}

fn contract_and_fixtures(plugin_root: &Path) -> Result<(), String> {
    let Some(root) = source_contract_root(plugin_root)? else {
        return Ok(());
    };
    validate_contract_root(root)
}

fn validate_contract_root(root: &Path) -> Result<(), String> {
    let contract =
        load(&root.join("packages/getcodexy/contracts/component-installation-contract.json"))?;
    let fixtures =
        load(&root.join("packages/getcodexy/tests/fixtures/component-installation-cases.json"))?;
    let manifest =
        load(&root.join("packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"))?;
    check_contract(&contract)?;
    component_manifest::check(&manifest, &contract)?;
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

fn source_contract_root(plugin_root: &Path) -> Result<Option<&Path>, String> {
    let Some(root) = plugin_root.parent().and_then(Path::parent) else {
        return Ok(None);
    };
    if !root.join("packages/codexy-runtime/Cargo.toml").is_file() {
        return Ok(None);
    }
    if root.join("plugins/codexy") != plugin_root {
        return Err(
            "repository component contract requires the canonical plugins/codexy root".to_owned(),
        );
    }
    if !repository_markers(root) {
        return Ok(None);
    }
    Ok(Some(root))
}

fn repository_markers(root: &Path) -> bool {
    root.join(".git").exists()
        && root.join("AGENTS.md").is_file()
        && root.join(".agents/plugins/marketplace.json").is_file()
        && root.join("packages/getcodexy/pyproject.toml").is_file()
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
    let component_manifest = object(contract, "component_manifest")?;
    exact_map_string(
        component_manifest,
        "schema",
        "getcodexy.component-manifest.v1",
    )?;
    exact_map_string(
        component_manifest,
        "package_resource",
        "codexy_runtime_tools/component-manifest.json",
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
    for (command, usage) in [
        ("install", "getcodexy install [COMPONENT ...] [--json]"),
        ("update", "getcodexy update [COMPONENT ...] [--json]"),
        (
            "remove",
            "getcodexy remove COMPONENT [COMPONENT ...] [--json]",
        ),
        ("status", "getcodexy status [--json]"),
        ("doctor", "getcodexy doctor [--json]"),
        ("bootstrap", "getcodexy bootstrap [--json]"),
    ] {
        exact_map_string(object_value(commands, command)?, "usage", usage)?;
    }
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
    exact_map_string(output, "doctor_schema", "getcodexy.doctor.v1")?;
    exact_array_value(
        output.get("required_mutation_receipt_fields"),
        &[
            "schema",
            "operation_id",
            "command",
            "outcome",
            "requested_components",
            "resolved_components",
            "selection_before",
            "selection_after",
            "installed_components",
            "source_of_truth",
            "errors",
        ],
        "required_mutation_receipt_fields",
    )?;
    exact_array_value(
        output.get("required_status_fields"),
        &[
            "schema",
            "command",
            "outcome",
            "inventory",
            "inventory_consistency",
            "selected_components",
            "installed_components",
            "source_of_truth",
            "errors",
        ],
        "required_status_fields",
    )?;
    exact_array_value(
        output.get("required_doctor_fields"),
        &[
            "schema",
            "command",
            "outcome",
            "inventory",
            "inventory_consistency",
            "host_readiness",
            "component_health",
            "source_of_truth",
            "errors",
        ],
        "required_doctor_fields",
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "getcodexy_component_contract_tests.rs"]
mod tests;
