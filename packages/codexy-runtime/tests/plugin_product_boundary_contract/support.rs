use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::model::{Product, SurfaceRecord};
use crate::support::TestResult;

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
    Ok(serde_json::from_str(&std::fs::read_to_string(
        root.join("docs/plugin-product-boundary.json"),
    )?)?)
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
pub(super) fn agent_requires_github_skill(
    root: &Path,
    source: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let agent: toml::Value = toml::from_str(&std::fs::read_to_string(root.join(source))?)?;
    Ok(agent
        .get("developer_instructions")
        .and_then(toml::Value::as_str)
        .is_some_and(|instructions| {
            instructions.split_whitespace().any(|word| {
                word.trim_matches(|character: char| {
                    !character.is_ascii_alphanumeric() && character != '-'
                })
                .eq("git-workflow")
            })
        }))
}
pub(super) fn validate_selector(root: &Path, path: &str, selector: &str) -> TestResult {
    let registrations: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(path))?)?;
    if selector.is_empty()
        || path != "plugins/codexy-devtools/.mcp.json"
        || registrations[selector].is_null()
    {
        return Err(format!("stale selector: {path}#{selector}").into());
    }
    Ok(())
}
pub(super) fn registration_selectors(
    root: &Path,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let registrations: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy-devtools/.mcp.json"),
    )?)?;
    Ok(registrations
        .as_object()
        .ok_or("MCP registrations must be an object")?
        .keys()
        .map(|key| format!("plugins/codexy-devtools/.mcp.json#{key}"))
        .collect())
}
