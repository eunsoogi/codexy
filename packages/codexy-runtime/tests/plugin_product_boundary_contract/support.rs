use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::model::{Product, SurfaceRecord, Topology};
use crate::support::TestResult;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MetadataContract {
    #[serde(rename = "schema")]
    _schema: String,
    #[serde(rename = "products")]
    _products: Vec<Product>,
    #[serde(rename = "repositoryTopology")]
    _repository_topology: Topology,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SurfaceSidecar {
    schema: String,
    target: String,
    surface_records: Vec<SurfaceRecord>,
}

pub(super) const SURFACE_SIDECARS: [(&str, &str); 4] = [
    (
        "docs/plugin-product-boundary-core.json",
        "codexy",
    ),
    (
        "docs/plugin-product-boundary-github.json",
        "codexy-github",
    ),
    (
        "docs/plugin-product-boundary-devtools.json",
        "codexy-devtools",
    ),
    (
        "docs/plugin-product-boundary-repository.json",
        "repository-only",
    ),
];

pub(super) fn unique_products(
    products: &[Product],
) -> Result<BTreeMap<&str, &Product>, Box<dyn std::error::Error>> {
    let mut unique = BTreeMap::new();
    for product in products {
        if unique.insert(product.id.as_str(), product).is_some() {
            return Err("duplicate product".into());
        }
    }
    Ok(unique)
}
pub(super) fn contract(root: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let metadata_text = std::fs::read_to_string(root.join("docs/plugin-product-boundary.json"))?;
    let _: MetadataContract = serde_json::from_str(&metadata_text)?;
    let mut contract: serde_json::Value = serde_json::from_str(&metadata_text)?;
    let mut records = Vec::new();
    for (path, expected_target) in SURFACE_SIDECARS {
        let sidecar_text = std::fs::read_to_string(root.join(path))?;
        let typed: SurfaceSidecar = serde_json::from_str(&sidecar_text)?;
        if typed.schema != "codexy-plugin-product-boundary-surfaces/v1"
            || typed.target != expected_target
        {
            return Err(format!("invalid product-boundary sidecar: {path}").into());
        }
        if typed.surface_records.is_empty() {
            return Err(format!("empty product-boundary sidecar: {path}").into());
        }
        let sidecar: serde_json::Value = serde_json::from_str(&sidecar_text)?;
        let sidecar_records = sidecar["surfaceRecords"]
            .as_array()
            .ok_or("sidecar surfaceRecords must be an array")?;
        for record in sidecar_records {
            if record["target"].as_str() != Some(expected_target) {
                return Err(format!("sidecar target mismatch: {path}").into());
            }
            records.push(record.clone());
        }
    }
    contract["surfaceRecords"] = serde_json::Value::Array(records);
    Ok(contract)
}

pub(super) fn reject_unknown_wrapper_fields(root: &Path) -> TestResult {
    let mut metadata: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("docs/plugin-product-boundary.json"),
    )?)?;
    metadata["surfaceRecords"] = serde_json::json!([]);
    if serde_json::from_value::<MetadataContract>(metadata).is_ok() {
        return Err("metadata accepted duplicate surfaceRecords authority".into());
    }
    let mut sidecar: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(SURFACE_SIDECARS[0].0),
    )?)?;
    sidecar["unexpected"] = serde_json::json!(true);
    if serde_json::from_value::<SurfaceSidecar>(sidecar).is_ok() {
        return Err("sidecar accepted an unknown wrapper field".into());
    }
    Ok(())
}
pub(super) fn product<'a>(value: &'a mut serde_json::Value, id: &str) -> &'a mut serde_json::Value {
    value["products"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["id"] == id)
        .unwrap()
}
pub(super) fn record<'a>(value: &'a mut serde_json::Value, id: &str) -> &'a mut serde_json::Value {
    value["surfaceRecords"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["id"] == id)
        .unwrap()
}
pub(super) fn assert_sources(records: &[SurfaceRecord], id: &str, expected: &[&str]) -> TestResult {
    let actual = records
        .iter()
        .find(|record| record.id == id)
        .ok_or("missing surface record")?
        .sources
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actual != expected.iter().copied().collect() {
        return Err(format!("source mismatch for {id}").into());
    }
    Ok(())
}
pub(super) fn validate_python_file(
    root: &Path,
    source: &str,
    target: &str,
    owned: &BTreeMap<&str, &str>,
) -> TestResult {
    for line in std::fs::read_to_string(root.join(source))?.lines() {
        validate_import(source, target, line, owned)?;
    }
    Ok(())
}
pub(super) fn validate_import(
    source: &str,
    target: &str,
    line: &str,
    owned: &BTreeMap<&str, &str>,
) -> TestResult {
    if let Some(module) = policy_import_module(line) {
        let package = source
            .split_once("/codexy_policy/")
            .map(|(prefix, _)| prefix)
            .or_else(|| source.rsplit_once('/').map(|(prefix, _)| prefix))
            .ok_or("policy source root")?;
        let dependency = format!("{package}/codexy_policy/{module}.py");
        let dependency_target = owned
            .get(dependency.as_str())
            .ok_or("missing Python import")?;
        if target == "codexy" && matches!(*dependency_target, "codexy-github" | "codexy-devtools") {
            return Err(format!("forbidden import {source} -> {dependency}").into());
        }
    }
    Ok(())
}
pub(super) fn policy_import_module(line: &str) -> Option<&str> {
    let line = line.trim();
    let tail = line
        .strip_prefix("from codexy_policy import ")
        .or_else(|| line.strip_prefix("import codexy_policy."))
        .or_else(|| line.strip_prefix("from codexy_policy."))
        .or_else(|| line.strip_prefix("from ."))?;
    let tail = tail.trim_start();
    let tail = tail.strip_prefix("import ").unwrap_or(tail);
    tail.split(|character: char| character == '.' || character == ',' || character.is_whitespace())
        .find(|part| !part.is_empty())
}
pub(super) fn validate_selector(root: &Path, path: &str, selector: &str) -> TestResult {
    let registrations: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(path))?)?;
    if selector.is_empty()
        || !matches!(
            path,
            "plugins/codexy-devtools/.mcp.json" | "plugins/codexy/.mcp.json"
        )
        || registrations[selector].is_null()
    {
        return Err(format!("stale selector: {path}#{selector}").into());
    }
    Ok(())
}
pub(super) fn registration_selectors(
    root: &Path,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let mut selectors = BTreeSet::new();
    for path in [
        "plugins/codexy-devtools/.mcp.json",
        "plugins/codexy/.mcp.json",
    ] {
        let registrations: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
            root.join(path),
        )?)?;
        selectors.extend(
            registrations
                .as_object()
                .ok_or("MCP registrations must be an object")?
                .keys()
                .map(|key| format!("{path}#{key}")),
        );
    }
    Ok(selectors)
}
