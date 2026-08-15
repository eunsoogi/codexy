use std::collections::{BTreeMap, BTreeSet};

use super::contract::validate_contract;
use super::inventory::files;
use super::support::{
    agent_requires_github_skill, contract, product, record, validate_import,
};
use crate::support::TestResult;

fn assert_invalid(root: &std::path::Path, value: &serde_json::Value) {
    assert!(
        validate_contract(root, value).is_err(),
        "invalid contract was accepted: {value}"
    );
}

#[test]
fn product_boundary_contract_owns_each_current_surface_once() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    validate_contract(root, &contract(root)?)?;
    let guide = std::fs::read_to_string(root.join("docs/plugin-product-boundary.md"))?;
    let headings = guide
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        headings,
        BTreeSet::from([
            "Current inventory mapping",
            "Forbidden work in this boundary freeze",
            "Public dependencies",
            "Public products and packaging",
            "Target destinations and dispositions"
        ])
    );
    Ok(())
}

#[test]
fn core_and_devtools_packages_keep_developer_tool_surfaces_separate() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let core_manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    assert!(core_manifest.get("mcpServers").is_none());
    for absent in [
        ".mcp.json",
        ".codex/lsp-client.json",
        "lsp",
        "mcp",
        "runtime-release.json",
    ] {
        assert!(
            !root.join("plugins/codexy").join(absent).exists(),
            "core retains devtools surface: {absent}"
        );
    }

    let devtools = root.join("plugins/codexy-devtools");
    let devtools_manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        devtools.join(".codex-plugin/plugin.json"),
    )?)?;
    assert_eq!(devtools_manifest["name"], "codexy-devtools");
    assert_eq!(devtools_manifest["mcpServers"], "./.mcp.json");
    assert!(devtools.join(".codex/lsp-client.json").is_file());
    assert!(devtools.join("lsp/server-catalog.toml").is_file());
    assert!(devtools.join("mcp/codexy-mcp-lsp").is_file());
    assert!(devtools.join("mcp/codexy-mcp-codegraph").is_file());
    Ok(())
}

#[test]
fn product_boundary_contract_rejects_invalid_surface_records() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let contract = contract(root)?;
    let mut duplicate = contract.clone();
    duplicate["surfaceRecords"]
        .as_array_mut()
        .unwrap()
        .push(contract["surfaceRecords"][0].clone());
    assert_invalid(root, &duplicate);
    let mut overlap = contract.clone();
    record(&mut overlap, "hooks.github")["sources"][0] =
        record(&mut contract.clone(), "hooks.instruction")["sources"][0].clone();
    assert_invalid(root, &overlap);
    let mut all_core = contract.clone();
    record(&mut all_core, "mcp.codegraph")["target"] = serde_json::json!("codexy");
    assert_invalid(root, &all_core);
    let mut stale = contract.clone();
    record(&mut stale, "mcp.codegraph")["sources"][0] =
        serde_json::json!("plugins/codexy/.mcp.json#missing");
    assert_invalid(root, &stale);
    let mut empty_selector = contract.clone();
    record(&mut empty_selector, "mcp.codegraph")["sources"][0] =
        serde_json::json!("plugins/codexy/.mcp.json#");
    assert_invalid(root, &empty_selector);
    let mut unknown = contract.clone();
    record(&mut unknown, "hooks.instruction")["target"] = serde_json::json!("unknown");
    assert_invalid(root, &unknown);
    let mut disposition = contract.clone();
    record(&mut disposition, "hooks.instruction")["disposition"] = serde_json::json!("unknown");
    assert_invalid(root, &disposition);
    let mut empty = contract.clone();
    record(&mut empty, "hooks.instruction")["sources"] = serde_json::json!([]);
    assert_invalid(root, &empty);
    let mut illegal_dependency = contract.clone();
    product(&mut illegal_dependency, "codexy-github")["dependsOn"] =
        serde_json::json!(["codexy-devtools"]);
    assert_invalid(root, &illegal_dependency);
    let mut missing_forbidden = contract.clone();
    product(&mut missing_forbidden, "codexy-github")["forbiddenDependencies"] =
        serde_json::json!([]);
    assert_invalid(root, &missing_forbidden);
    let mut omitted = contract.clone();
    omitted["surfaceRecords"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["id"] != "runtime.codegraph");
    assert_invalid(root, &omitted);
    let mut selector_overlap = contract.clone();
    record(&mut selector_overlap, "mcp.runtimes")["sources"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!("plugins/codexy/.mcp.json"));
    assert_invalid(root, &selector_overlap);
    let mut unknown_product = contract.clone();
    product(&mut unknown_product, "codexy")["id"] = serde_json::json!("other");
    assert_invalid(root, &unknown_product);
    let mut unknown_name = contract.clone();
    product(&mut unknown_name, "codexy")["publicName"] = serde_json::json!("Other");
    assert_invalid(root, &unknown_name);
    let mut unknown_root = contract.clone();
    product(&mut unknown_root, "codexy")["packageRoot"] = serde_json::json!("other");
    assert_invalid(root, &unknown_root);
    let mut parallel = contract.clone();
    parallel["currentSourceInventory"] = serde_json::json!({"all":"codexy"});
    assert_invalid(root, &parallel);
    let mut github_agent = contract.clone();
    record(&mut github_agent, "agents.github")["target"] = serde_json::json!("codexy");
    assert_invalid(root, &github_agent);
    assert!(agent_requires_github_skill(
        root,
        "plugins/codexy-github/agents/codexy-weaver.toml"
    )?);
    let owned = BTreeMap::from([
        ("plugins/codexy/hooks/codexy_policy/admission.py", "codexy"),
        (
            "plugins/codexy/hooks/codexy_policy/github.py",
            "codexy-github",
        ),
    ]);
    assert!(
        validate_import(
            "plugins/codexy/hooks/codexy_policy/admission.py",
            "codexy",
            "from .github import connector_admitted as admitted",
            &owned
        )
        .is_err()
    );
    assert!(
        validate_import(
            "plugins/codexy/hooks/codexy_policy/admission.py",
            "codexy",
            "from . import github",
            &owned
        )
        .is_err()
    );
    assert!(
        validate_import(
            "plugins/codexy/hooks/codexy_policy/admission.py",
            "codexy",
            "from .missing import evaluator as alias",
            &owned
        )
        .is_err()
    );
    assert!(
        validate_import(
            "plugins/codexy/hooks/codexy_policy/admission.py",
            "codexy",
            "import codexy_policy.github",
            &owned
        )
        .is_err()
    );
    assert!(
        validate_import(
            "plugins/codexy/hooks/codexy_policy/admission.py",
            "codexy",
            "from codexy_policy import github",
            &owned
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn governed_path_identities_are_portable_and_fail_closed() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let identities = files(root.join("plugins/codexy/hooks"))?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let expected = BTreeSet::from([
        "plugins/codexy/hooks/README.md".into(),
        "plugins/codexy/hooks/hooks.json".into(),
    ]);
    assert_eq!(
        identities
            .intersection(&expected)
            .cloned()
            .collect::<BTreeSet<_>>(),
        expected
    );
    let outside = tempfile::NamedTempFile::new()?;
    assert!(files(outside.path().to_path_buf()).is_err());
    Ok(())
}
