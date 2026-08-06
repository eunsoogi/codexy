use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::support::TestResult;

const PRODUCTS: [(&str, &str, &str, &[&str], &[&str]); 3] = [
    ("codexy", "Codexy", "plugins/codexy", &[], &["codexy-github", "codexy-devtools"]),
    ("codexy-github", "Codexy GitHub", "plugins/codexy-github", &["codexy"], &["codexy-devtools"]),
    ("codexy-devtools", "Codexy Devtools", "plugins/codexy-devtools", &["codexy"], &["codexy-github"]),
];
const TARGETS: [&str; 5] = ["codexy", "codexy-github", "codexy-devtools", "repository-only", "remove"];
const DISPOSITIONS: [&str; 5] = ["retain", "move", "merge", "split", "remove"];
const CATEGORIES: [&str; 10] = ["hooks", "skills", "agents", "mcp", "lsp", "assets", "validators", "workflows", "packaging", "public-entrypoints"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundaryContract { schema: String, products: Vec<Product>, repository_topology: Topology, surface_records: Vec<SurfaceRecord> }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Product { id: String, public_name: String, package_root: String, responsibility: String, depends_on: Vec<String>, forbidden_dependencies: Vec<String> }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Topology { repository_root: String, current_rust_runtime_root: String, future_rust_runtime_root: String, future_python_distribution_root: String, root_cargo_workspace: String, physical_migration: String }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SurfaceRecord { id: String, category: String, sources: Vec<String>, target: String, disposition: String }

#[test]
fn product_boundary_contract_owns_each_current_surface_once() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    validate_contract(root, &contract(root)?)?;
    let guide = std::fs::read_to_string(root.join("docs/plugin-product-boundary.md"))?;
    for required in ["codexy", "codexy-github", "codexy-devtools", "Forbidden dependencies", "Physical extraction"] { assert!(guide.contains(required), "guide misses {required}"); }
    Ok(())
}

#[test]
fn product_boundary_contract_rejects_invalid_surface_records() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")); let contract = contract(root)?;
    let mut duplicate = contract.clone(); duplicate["surfaceRecords"].as_array_mut().unwrap().push(contract["surfaceRecords"][0].clone()); assert_invalid(root, &duplicate);
    let mut overlap = contract.clone(); record(&mut overlap, "hooks.github")["sources"][0] = record(&mut contract.clone(), "hooks.instruction")["sources"][0].clone(); assert_invalid(root, &overlap);
    let mut all_core = contract.clone(); record(&mut all_core, "mcp.codegraph")["target"] = serde_json::json!("codexy"); assert_invalid(root, &all_core);
    let mut stale = contract.clone(); record(&mut stale, "mcp.codegraph")["sources"][0] = serde_json::json!("plugins/codexy/.mcp.json#missing"); assert_invalid(root, &stale);
    let mut empty_selector = contract.clone(); record(&mut empty_selector, "mcp.codegraph")["sources"][0] = serde_json::json!("plugins/codexy/.mcp.json#"); assert_invalid(root, &empty_selector);
    let mut unknown = contract.clone(); record(&mut unknown, "hooks.instruction")["target"] = serde_json::json!("unknown"); assert_invalid(root, &unknown);
    let mut disposition = contract.clone(); record(&mut disposition, "hooks.instruction")["disposition"] = serde_json::json!("unknown"); assert_invalid(root, &disposition);
    let mut empty = contract.clone(); record(&mut empty, "hooks.instruction")["sources"] = serde_json::json!([]); assert_invalid(root, &empty);
    let mut illegal_dependency = contract.clone(); product(&mut illegal_dependency, "codexy-github")["dependsOn"] = serde_json::json!(["codexy-devtools"]); assert_invalid(root, &illegal_dependency);
    let mut missing_forbidden = contract.clone(); product(&mut missing_forbidden, "codexy-github")["forbiddenDependencies"] = serde_json::json!([]); assert_invalid(root, &missing_forbidden);
    let mut omitted = contract.clone(); omitted["surfaceRecords"].as_array_mut().unwrap().retain(|entry| entry["id"] != "runtime.codegraph"); assert_invalid(root, &omitted);
    let mut selector_overlap = contract.clone(); record(&mut selector_overlap, "mcp.runtimes")["sources"].as_array_mut().unwrap().push(serde_json::json!("plugins/codexy/.mcp.json")); assert_invalid(root, &selector_overlap);
    let mut unknown_product = contract.clone(); product(&mut unknown_product, "codexy")["id"] = serde_json::json!("other"); assert_invalid(root, &unknown_product);
    let mut unknown_name = contract.clone(); product(&mut unknown_name, "codexy")["publicName"] = serde_json::json!("Other"); assert_invalid(root, &unknown_name);
    let mut unknown_root = contract.clone(); product(&mut unknown_root, "codexy")["packageRoot"] = serde_json::json!("other"); assert_invalid(root, &unknown_root);
    let mut parallel = contract.clone(); parallel["currentSourceInventory"] = serde_json::json!({"all":"codexy"}); assert_invalid(root, &parallel);
    let owned = BTreeMap::from([("plugins/codexy/hooks/codexy_policy/admission.py", "codexy-github")]);
    assert!(validate_import("plugins/codexy/hooks/codexy-admission.py", "codexy", "from codexy_policy.admission import evaluate", &owned).is_err());
    assert!(validate_import("plugins/codexy/hooks/codexy_policy/admission.py", "codexy", "from .missing import evaluator as alias", &owned).is_err());
    Ok(())
}

fn validate_contract(root: &Path, value: &serde_json::Value) -> TestResult { validate_typed(root, &serde_json::from_value(value.clone())?) }

fn validate_typed(root: &Path, contract: &BoundaryContract) -> TestResult {
    if contract.schema != "codexy-plugin-product-boundary/v1" { return Err("unexpected product-boundary schema".into()); }
    let products = unique_products(&contract.products)?;
    if products.len() != PRODUCTS.len() { return Err("unknown product".into()); }
    for (id, name, package_root, depends_on, forbidden) in PRODUCTS {
        let product = products.get(id).ok_or("missing product")?;
        if product.public_name != name || product.package_root != package_root || product.depends_on.iter().map(String::as_str).collect::<Vec<_>>() != depends_on || product.forbidden_dependencies.iter().map(String::as_str).collect::<Vec<_>>() != forbidden { return Err(format!("product matrix mismatch: {id}").into()); }
    }
    if contract.repository_topology.repository_root != "." || contract.repository_topology.current_rust_runtime_root != "Cargo.toml" || contract.repository_topology.future_rust_runtime_root != "packages/codexy-runtime" || contract.repository_topology.future_python_distribution_root != "packages/getcodexy" || contract.repository_topology.root_cargo_workspace != "not-required" || contract.repository_topology.physical_migration != "out-of-scope" { return Err("repository topology changed".into()); }
    if products["codexy"].responsibility.to_lowercase().contains("codegraph") || products["codexy"].responsibility.to_lowercase().contains("lsp") { return Err("core responsibility contradicts devtools ownership".into()); }
    if !products["codexy-github"].responsibility.to_lowercase().contains("github") || !products["codexy-devtools"].responsibility.to_lowercase().contains("codegraph") || !products["codexy-devtools"].responsibility.to_lowercase().contains("lsp") { return Err("product responsibility mismatch".into()); }
    validate_records(root, &contract.surface_records)
}

fn validate_records(root: &Path, records: &[SurfaceRecord]) -> TestResult {
    let mut ids = BTreeSet::new(); let mut categories = BTreeSet::new(); let mut logical = BTreeSet::new(); let mut covered = BTreeMap::new(); let mut whole: BTreeSet<String> = BTreeSet::new(); let mut selector_paths: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for record in records {
        if !ids.insert(record.id.as_str()) || !CATEGORIES.contains(&record.category.as_str()) || !TARGETS.contains(&record.target.as_str()) || !DISPOSITIONS.contains(&record.disposition.as_str()) || record.sources.is_empty() || (record.target == "remove") != (record.disposition == "remove") { return Err(format!("invalid record {}", record.id).into()); }
        categories.insert(record.category.as_str());
        for source in &record.sources {
            if !logical.insert(source.as_str()) { return Err(format!("duplicate logical source: {source}").into()); }
            let (path, selector) = source.split_once('#').map_or((source.as_str(), None), |(path, selector)| (path, Some(selector)));
            if path.is_empty() || !root.join(path).exists() { return Err(format!("surface source is absent: {source}").into()); }
            if let Some(selector) = selector { validate_selector(root, path, selector)?; selector_paths.entry(path.into()).or_default().insert(selector.into()); }
            else { whole.insert(path.into()); for file in files(root.join(path))? { if let Some(owner) = covered.insert(file.clone(), record.id.as_str()) { return Err(format!("overlapping source {file}: {owner}, {}", record.id).into()); } } }
        }
    }
    if categories != CATEGORIES.into_iter().collect() { return Err("surface category omitted".into()); }
    if whole.iter().any(|path| selector_paths.contains_key(path)) { return Err("selector/file overlap".into()); }
    for (id, target) in [("hooks.github", "codexy-github"), ("hooks.policy-core", "codexy"), ("skills.github", "codexy-github"), ("mcp.codegraph", "codexy-devtools"), ("mcp.lsp", "codexy-devtools"), ("runtime.codegraph", "codexy-devtools"), ("runtime.lsp", "codexy-devtools"), ("runtime.entrypoints", "codexy-devtools"), ("assets.repository", "repository-only"), ("assets.plugin", "codexy"), ("mcp.grep-app", "remove")] { if records.iter().find(|record| record.id == id).map(|record| record.target.as_str()) != Some(target) { return Err(format!("target mismatch for {id}").into()); } }
    for required in ["hooks.instruction", "hooks.policy-core", "skills.core", "agents.specialists", "mcp.codegraph", "mcp.lsp", "mcp.grep-app", "runtime.codegraph", "runtime.lsp", "runtime.entrypoints", "repository.governance", "repository.packaging", "assets.repository", "assets.plugin", "public.core"] { if !ids.contains(required) { return Err(format!("missing required surface record: {required}").into()); } }
    assert_sources(records, "hooks.policy-core", &["plugins/codexy/hooks/codexy_policy/__init__.py","plugins/codexy/hooks/codexy_policy/admission.py","plugins/codexy/hooks/codexy_policy/body.py","plugins/codexy/hooks/codexy_policy/executable_identity.py","plugins/codexy/hooks/codexy_policy/execution_context.py","plugins/codexy/hooks/codexy_policy/filesystem_state.py","plugins/codexy/hooks/codexy_policy/git_command.py","plugins/codexy/hooks/codexy_policy/git_options.py","plugins/codexy/hooks/codexy_policy/git_runtime_config.py","plugins/codexy/hooks/codexy_policy/github.py","plugins/codexy/hooks/codexy_policy/github_alias.py","plugins/codexy/hooks/codexy_policy/github_api.py","plugins/codexy/hooks/codexy_policy/github_target.py","plugins/codexy/hooks/codexy_policy/graphql.py","plugins/codexy/hooks/codexy_policy/graphql_parser.py","plugins/codexy/hooks/codexy_policy/invocation.py","plugins/codexy/hooks/codexy_policy/invocation_wrappers.py","plugins/codexy/hooks/codexy_policy/merge.py","plugins/codexy/hooks/codexy_policy/pull_request.py","plugins/codexy/hooks/codexy_policy/repository.py","plugins/codexy/hooks/codexy_policy/shell.py","plugins/codexy/hooks/codexy_policy/shell_builtins.py","plugins/codexy/hooks/codexy_policy/shell_context.py","plugins/codexy/hooks/codexy_policy/shell_groups.py","plugins/codexy/hooks/codexy_policy/shell_sequence.py","plugins/codexy/hooks/codexy_policy/titles.py","plugins/codexy/hooks/codexy_policy/wrappers.py"])?;
    let expected_selectors = registration_selectors(root)?; let actual_selectors: BTreeSet<String> = selector_paths.into_iter().flat_map(|(path, selectors)| selectors.into_iter().map(move |selector| format!("{path}#{selector}"))).collect();
    if actual_selectors != expected_selectors { return Err("MCP registration coverage changed".into()); }
    let governed = governed_universe(root)?;
    if covered.keys().cloned().collect::<BTreeSet<_>>() != governed { return Err("governed universe is not exact-covered".into()); }
    let owned = records.iter().flat_map(|record| record.sources.iter().filter(|source| source.ends_with(".py")).map(|source| (source.as_str(), record.target.as_str()))).collect();
    for (&source, &target) in &owned { validate_python_file(root, source, target, &owned)?; }
    Ok(())
}

fn unique_products(products: &[Product]) -> Result<BTreeMap<&str, &Product>, Box<dyn std::error::Error>> { let mut unique = BTreeMap::new(); for product in products { if unique.insert(product.id.as_str(), product).is_some() { return Err("duplicate product".into()); } } Ok(unique) }
fn contract(root: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> { Ok(serde_json::from_str(&std::fs::read_to_string(root.join("docs/plugin-product-boundary.json"))?)?) }
fn product<'a>(value: &'a mut serde_json::Value, id: &str) -> &'a mut serde_json::Value { value["products"].as_array_mut().unwrap().iter_mut().find(|entry| entry["id"] == id).unwrap() }
fn record<'a>(value: &'a mut serde_json::Value, id: &str) -> &'a mut serde_json::Value { value["surfaceRecords"].as_array_mut().unwrap().iter_mut().find(|entry| entry["id"] == id).unwrap() }
fn assert_invalid(root: &Path, value: &serde_json::Value) { assert!(validate_contract(root, value).is_err(), "invalid contract was accepted: {value}"); }
fn assert_sources(records: &[SurfaceRecord], id: &str, expected: &[&str]) -> TestResult { let actual = records.iter().find(|record| record.id == id).ok_or("missing surface record")?.sources.iter().map(String::as_str).collect::<BTreeSet<_>>(); if actual != expected.iter().copied().collect() { return Err(format!("source mismatch for {id}").into()); } Ok(()) }
fn validate_python_file(root: &Path, source: &str, target: &str, owned: &BTreeMap<&str, &str>) -> TestResult { for line in std::fs::read_to_string(root.join(source))?.lines() { validate_import(source, target, line, owned)?; } Ok(()) }
fn validate_import(source: &str, target: &str, line: &str, owned: &BTreeMap<&str, &str>) -> TestResult { let module = line.trim().strip_prefix("from .").or_else(|| line.trim().strip_prefix("from codexy_policy.")).and_then(|tail| tail.split_whitespace().next()); if let Some(module) = module { let dependency = format!("plugins/codexy/hooks/codexy_policy/{module}.py"); let dependency_target = owned.get(dependency.as_str()).ok_or("missing Python import")?; if target == "codexy" && matches!(*dependency_target, "codexy-github" | "codexy-devtools") { return Err(format!("forbidden import {source} -> {dependency}").into()); } } Ok(()) }
fn validate_selector(root: &Path, path: &str, selector: &str) -> TestResult { let registrations: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join(path))?)?; if selector.is_empty() || path != "plugins/codexy/.mcp.json" || registrations[selector].is_null() { return Err(format!("stale selector: {path}#{selector}").into()); } Ok(()) }
fn registration_selectors(root: &Path) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> { let registrations: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("plugins/codexy/.mcp.json"))?)?; Ok(registrations.as_object().ok_or("MCP registrations must be an object")?.keys().map(|key| format!("plugins/codexy/.mcp.json#{key}")).collect()) }
fn governed_universe(root: &Path) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> { let mut governed = BTreeSet::new(); for path in ["plugins/codexy/hooks","plugins/codexy/skills","plugins/codexy/agents","plugins/codexy/mcp","plugins/codexy/lsp","assets","plugins/codexy/assets","scripts","src/validation","tests",".github/workflows","src/codegraph","src/lsp"] { governed.extend(files(root.join(path))?); } for path in ["src/mcp.rs","src/bin/codexy-mcp-codegraph.rs","src/bin/codexy-mcp-lsp.rs","Cargo.toml","Cargo.lock","rust-toolchain.toml","rustfmt.toml","clippy.toml","packages/getcodexy/pyproject.toml","plugins/codexy/runtime-release.json",".agents/plugins/marketplace.json",".agents/plugins/release-publish-contract.json",".agents/plugins/runtime-activation.json","plugins/codexy/.codex/lsp-client.json","plugins/codexy/.codex-plugin/plugin.json","plugins/codexy/agents/openai.yaml","README.md","README.ko.md"] { governed.extend(files(root.join(path))?); } Ok(governed) }
fn files(path: PathBuf) -> Result<Vec<String>, Box<dyn std::error::Error>> { if path.is_file() { return Ok(vec![path.to_string_lossy().strip_prefix(&format!("{}/", env!("CARGO_MANIFEST_DIR"))).unwrap_or_default().to_owned()]); } let mut found = Vec::new(); for entry in std::fs::read_dir(path)? { found.extend(files(entry?.path())?); } Ok(found) }
