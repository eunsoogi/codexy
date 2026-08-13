use serde_json::Value;

use super::schema::{COMPONENTS, exact_array_value, object, object_value};

pub(super) fn check(manifest: &Value, contract: &Value) -> Result<(), String> {
    if manifest.get("schema").and_then(Value::as_str) != Some("getcodexy.component-manifest.v1") {
        return Err("component manifest schema is invalid".to_owned());
    }
    let marketplace = object(manifest, "marketplace")?;
    if marketplace.get("name").and_then(Value::as_str) != Some("codexy")
        || marketplace.get("source").and_then(Value::as_str)
            != Some("https://github.com/eunsoogi/codexy.git")
    {
        return Err("component manifest marketplace is invalid".to_owned());
    }
    let components = manifest
        .get("components")
        .and_then(Value::as_array)
        .ok_or_else(|| "component manifest components must be an array".to_owned())?;
    if components.len() != COMPONENTS.len() {
        return Err("component manifest components must list every component".to_owned());
    }
    let products = object(contract, "component_products")?;
    let dependencies = object(contract, "dependencies")?;
    for (entry, component) in components.iter().zip(COMPONENTS) {
        let entry = entry
            .as_object()
            .ok_or_else(|| "component manifest component must be an object".to_owned())?;
        if entry.get("id").and_then(Value::as_str) != Some(component)
            || entry.get("plugin").and_then(Value::as_str)
                != products.get(*component).and_then(Value::as_str)
        {
            return Err(format!(
                "component manifest {component} does not match the contract"
            ));
        }
        exact_array_value(
            entry.get("dependencies"),
            dependencies
                .get(*component)
                .and_then(Value::as_array)
                .ok_or_else(|| format!("contract dependencies.{component} must be an array"))?
                .iter()
                .map(Value::as_str)
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| format!("contract dependencies.{component} must be strings"))?
                .as_slice(),
            &format!("component manifest dependencies.{component}"),
        )?;
        let asset = object_value(entry, "asset")?;
        if asset.get("pluginId").and_then(Value::as_str)
            != entry
                .get("plugin")
                .and_then(Value::as_str)
                .map(|plugin| format!("{plugin}@codexy"))
                .as_deref()
        {
            return Err(format!(
                "component manifest {component} asset plugin ID is invalid"
            ));
        }
    }
    Ok(())
}
