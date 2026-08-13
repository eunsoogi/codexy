use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::schema::{COMPONENTS, exact_array_value, object, object_value};

const MANIFEST_FIELDS: &[&str] = &[
    "schema",
    "marketplace",
    "domainErrors",
    "components",
    "compatibleCombinations",
];
const COMPONENT_FIELDS: &[&str] = &["id", "plugin", "version", "dependencies", "asset"];
const ASSET_FIELDS: &[&str] = &["pluginId", "packageRoot", "requiredPaths"];
const MAX_SEMVER_COMPONENT: u32 = 2_147_483_647;

pub(super) fn check(manifest: &Value, contract: &Value) -> Result<(), String> {
    exact_fields(manifest.as_object(), MANIFEST_FIELDS, "component manifest")?;
    if manifest.get("schema").and_then(Value::as_str) != Some("getcodexy.component-manifest.v1") {
        return Err("component manifest schema is invalid".to_owned());
    }
    let marketplace = object(manifest, "marketplace")?;
    exact_fields(
        Some(marketplace),
        &["name", "source"],
        "component manifest marketplace",
    )?;
    if marketplace.get("name").and_then(Value::as_str) != Some("codexy")
        || marketplace.get("source").and_then(Value::as_str)
            != Some("https://github.com/eunsoogi/codexy.git")
    {
        return Err("component manifest marketplace is invalid".to_owned());
    }
    let products = object(contract, "component_products")?;
    let dependencies = object(contract, "dependencies")?;
    let errors = object(contract, "domain_errors")?;
    check_errors(manifest.get("domainErrors"), errors)?;
    let components = manifest
        .get("components")
        .and_then(Value::as_array)
        .ok_or_else(|| "component manifest components must be an array".to_owned())?;
    if components.len() != COMPONENTS.len() {
        return Err("component manifest components must list every component".to_owned());
    }
    let versions = components
        .iter()
        .zip(COMPONENTS)
        .map(|(entry, id)| check_component(entry, id, products, dependencies))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if versions.len() != 1 {
        return Err("component manifest component versions must be lockstep".to_owned());
    }
    check_combinations(
        manifest.get("compatibleCombinations"),
        versions.into_iter().next().unwrap(),
    )
}

fn check_errors(value: Option<&Value>, expected: &Map<String, Value>) -> Result<(), String> {
    let errors = value
        .and_then(Value::as_object)
        .ok_or_else(|| "component manifest domain errors must be an object".to_owned())?;
    if errors.keys().collect::<BTreeSet<_>>() != expected.keys().collect()
        || errors
            .values()
            .any(|value| value.as_str().is_none_or(str::is_empty))
    {
        return Err("component manifest domain errors must project the public contract".to_owned());
    }
    Ok(())
}

fn check_component(
    entry: &Value,
    component: &str,
    products: &Map<String, Value>,
    dependencies: &Map<String, Value>,
) -> Result<String, String> {
    let entry = entry
        .as_object()
        .ok_or_else(|| "component manifest component must be an object".to_owned())?;
    exact_fields(
        Some(entry),
        COMPONENT_FIELDS,
        "component manifest component",
    )?;
    let plugin = products
        .get(component)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("contract component_products.{component} must be a string"))?;
    let version = entry
        .get("version")
        .and_then(Value::as_str)
        .filter(|value| semver(value))
        .ok_or_else(|| format!("component manifest {component} version is invalid"))?;
    if entry.get("id").and_then(Value::as_str) != Some(component)
        || entry.get("plugin").and_then(Value::as_str) != Some(plugin)
    {
        return Err(format!(
            "component manifest {component} does not match the contract"
        ));
    }
    exact_array_value(
        entry.get("dependencies"),
        dependencies
            .get(component)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("contract dependencies.{component} must be an array"))?
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("contract dependencies.{component} must be strings"))?
            .as_slice(),
        &format!("component manifest dependencies.{component}"),
    )?;
    check_asset(object_value(entry, "asset")?, plugin, component)?;
    Ok(version.to_owned())
}

fn check_asset(asset: &Map<String, Value>, plugin: &str, component: &str) -> Result<(), String> {
    exact_fields(Some(asset), ASSET_FIELDS, "component manifest asset")?;
    let paths = asset
        .get("requiredPaths")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!("component manifest {component} asset required paths are invalid")
        })?;
    let paths = paths
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            format!("component manifest {component} asset required paths are invalid")
        })?;
    if asset.get("pluginId").and_then(Value::as_str) != Some(&format!("{plugin}@codexy"))
        || asset.get("packageRoot").and_then(Value::as_str) != Some(&format!("plugins/{plugin}"))
        || paths.is_empty()
        || paths.iter().any(|path| {
            path.is_empty() || path.starts_with('/') || path.split('/').any(|part| part == "..")
        })
        || paths.iter().collect::<BTreeSet<_>>().len() != paths.len()
    {
        return Err(format!(
            "component manifest {component} asset is not canonical"
        ));
    }
    Ok(())
}

fn check_combinations(value: Option<&Value>, version: String) -> Result<(), String> {
    let combinations = value
        .and_then(Value::as_array)
        .ok_or_else(|| "component manifest compatible combinations must be an array".to_owned())?;
    let expected = [
        vec![],
        vec!["core"],
        vec!["core", "github"],
        vec!["core", "devtools"],
        vec!["core", "github", "devtools"],
    ];
    if combinations.len() != expected.len() {
        return Err("component manifest compatible combinations are incomplete".to_owned());
    }
    for (entry, expected) in combinations.iter().zip(expected) {
        let entry = entry
            .as_object()
            .ok_or_else(|| "component manifest compatibility must be an object".to_owned())?;
        exact_fields(
            Some(entry),
            &["components", "version"],
            "component manifest compatibility",
        )?;
        if entry.get("version").and_then(Value::as_str) != Some(&version) {
            return Err("component manifest compatibility version is invalid".to_owned());
        }
        exact_array_value(
            entry.get("components"),
            &expected,
            "component manifest compatibility components",
        )?;
    }
    Ok(())
}

fn exact_fields(
    value: Option<&Map<String, Value>>,
    expected: &[&str],
    label: &str,
) -> Result<(), String> {
    let value = value.ok_or_else(|| format!("{label} must be an object"))?;
    if value.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != expected.iter().copied().collect()
    {
        return Err(format!("{label} has an invalid shape"));
    }
    Ok(())
}

fn semver(value: &str) -> bool {
    value.split('.').count() == 3
        && value.split('.').all(|part| {
            !part.is_empty()
                && (part == "0" || !part.starts_with('0'))
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && part
                    .parse::<u32>()
                    .is_ok_and(|number| number <= MAX_SEMVER_COMPONENT)
        })
}
