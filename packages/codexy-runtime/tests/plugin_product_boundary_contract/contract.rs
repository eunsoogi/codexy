use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::inventory::{files, governed_universe, record_matrix};
use super::model::{BoundaryContract, CATEGORIES, DISPOSITIONS, PRODUCTS, SurfaceRecord, TARGETS};
use super::support::{
    agent_requires_github_skill, assert_sources, registration_selectors, unique_products,
    validate_python_file, validate_selector,
};
use crate::support::TestResult;

pub(super) fn validate_contract(root: &Path, value: &serde_json::Value) -> TestResult {
    validate_typed(root, &serde_json::from_value(value.clone())?)
}

pub(super) fn validate_typed(root: &Path, contract: &BoundaryContract) -> TestResult {
    if contract.schema != "codexy-plugin-product-boundary/v1" {
        return Err("unexpected product-boundary schema".into());
    }
    let products = unique_products(&contract.products)?;
    if products.len() != PRODUCTS.len() {
        return Err("unknown product".into());
    }
    for (id, name, package_root, depends_on, forbidden) in PRODUCTS {
        let product = products.get(id).ok_or("missing product")?;
        if product.public_name != name
            || product.package_root != package_root
            || product
                .depends_on
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != depends_on
            || product
                .forbidden_dependencies
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != forbidden
        {
            return Err(format!("product matrix mismatch: {id}").into());
        }
    }
    if contract.repository_topology.repository_root != "."
        || contract.repository_topology.current_rust_runtime_root != "packages/codexy-runtime"
        || contract.repository_topology.future_rust_runtime_root != "packages/codexy-runtime"
        || contract.repository_topology.future_python_distribution_root != "packages/getcodexy"
        || contract.repository_topology.root_cargo_workspace != "not-required"
        || contract.repository_topology.physical_migration != "complete"
    {
        return Err("repository topology changed".into());
    }
    if products["codexy"]
        .responsibility
        .to_lowercase()
        .contains("codegraph")
        || products["codexy"]
            .responsibility
            .to_lowercase()
            .contains("lsp")
    {
        return Err("core responsibility contradicts devtools ownership".into());
    }
    if !products["codexy-github"]
        .responsibility
        .to_lowercase()
        .contains("github")
        || !products["codexy-devtools"]
            .responsibility
            .to_lowercase()
            .contains("codegraph")
        || !products["codexy-devtools"]
            .responsibility
            .to_lowercase()
            .contains("lsp")
    {
        return Err("product responsibility mismatch".into());
    }
    validate_records(root, &contract.surface_records)
}

pub(super) fn validate_records(root: &Path, records: &[SurfaceRecord]) -> TestResult {
    let mut ids = BTreeSet::new();
    let mut categories = BTreeSet::new();
    let mut logical = BTreeSet::new();
    let mut covered = BTreeMap::new();
    let mut whole: BTreeSet<String> = BTreeSet::new();
    let mut selector_paths: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for record in records {
        if !ids.insert(record.id.as_str())
            || !CATEGORIES.contains(&record.category.as_str())
            || !TARGETS.contains(&record.target.as_str())
            || !DISPOSITIONS.contains(&record.disposition.as_str())
            || record.sources.is_empty()
            || (record.target == "remove") != (record.disposition == "remove")
        {
            return Err(format!("invalid record {}", record.id).into());
        }
        categories.insert(record.category.as_str());
        for source in &record.sources {
            if !logical.insert(source.as_str()) {
                return Err(format!("duplicate logical source: {source}").into());
            }
            let (path, selector) = source
                .split_once('#')
                .map_or((source.as_str(), None), |(path, selector)| {
                    (path, Some(selector))
                });
            if path.is_empty() || !root.join(path).exists() {
                return Err(format!("surface source is absent: {source}").into());
            }
            if let Some(selector) = selector {
                validate_selector(root, path, selector)?;
                selector_paths
                    .entry(path.into())
                    .or_default()
                    .insert(selector.into());
            } else {
                whole.insert(path.into());
                for file in files(root.join(path))? {
                    if let Some(owner) = covered.insert(file.clone(), record.id.as_str()) {
                        return Err(
                            format!("overlapping source {file}: {owner}, {}", record.id).into()
                        );
                    }
                }
            }
        }
    }
    if categories != CATEGORIES.into_iter().collect() {
        return Err("surface category omitted".into());
    }
    if whole.iter().any(|path| selector_paths.contains_key(path)) {
        return Err("selector/file overlap".into());
    }
    for (id, target) in [
        ("hooks.github", "codexy-github"),
        ("hooks.policy-core", "codexy"),
        ("skills.github", "codexy-github"),
        ("mcp.codegraph", "codexy-devtools"),
        ("mcp.lsp", "codexy-devtools"),
        ("runtime.codegraph", "codexy-devtools"),
        ("runtime.lsp", "codexy-devtools"),
        ("runtime.entrypoints", "codexy-devtools"),
        ("assets.repository", "repository-only"),
        ("assets.plugin", "codexy"),
    ] {
        if records
            .iter()
            .find(|record| record.id == id)
            .map(|record| record.target.as_str())
            != Some(target)
        {
            return Err(format!("target mismatch for {id}").into());
        }
    }
    let matrix = record_matrix();
    if ids != matrix.keys().copied().collect() {
        return Err("surface-record matrix changed".into());
    }
    for (id, (target, disposition)) in matrix {
        let record = records
            .iter()
            .find(|record| record.id == id)
            .ok_or("missing record")?;
        if (record.target.as_str(), record.disposition.as_str()) != (target, disposition) {
            return Err(format!("record matrix mismatch: {id}").into());
        }
    }
    assert_sources(
        records,
        "hooks.policy-core",
        &[
            "plugins/codexy/hooks/codexy_policy/__init__.py",
            "plugins/codexy/hooks/codexy_policy/envelope.py",
            "plugins/codexy/hooks/codexy_policy/thread_delivery.py",
        ],
    )?;
    let expected_selectors = registration_selectors(root)?;
    let actual_selectors: BTreeSet<String> = selector_paths
        .into_iter()
        .flat_map(|(path, selectors)| {
            selectors
                .into_iter()
                .map(move |selector| format!("{path}#{selector}"))
        })
        .collect();
    if actual_selectors != expected_selectors {
        return Err("MCP registration coverage changed".into());
    }
    let governed = governed_universe(root)?;
    let observed = covered.keys().cloned().collect::<BTreeSet<_>>();
    if observed != governed {
        return Err(format!(
            "governed universe is not exact-covered; missing={:?}; extra={:?}",
            governed.difference(&observed).collect::<Vec<_>>(),
            observed.difference(&governed).collect::<Vec<_>>()
        )
        .into());
    }
    let owned = records
        .iter()
        .flat_map(|record| {
            record
                .sources
                .iter()
                .filter(|source| source.ends_with(".py"))
                .map(|source| (source.as_str(), record.target.as_str()))
        })
        .collect();
    for (&source, &target) in &owned {
        validate_python_file(root, source, target, &owned)?;
    }
    for record in records {
        for source in &record.sources {
            if source.ends_with(".toml")
                && agent_requires_github_skill(root, source)?
                && record.target == "codexy"
            {
                return Err(format!("core agent requires GitHub skill: {source}").into());
            }
        }
    }
    Ok(())
}
