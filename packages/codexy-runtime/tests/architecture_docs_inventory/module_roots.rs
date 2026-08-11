use crate::support::TestResult;

const ARCHITECTURE_PATHS: &[(&str, &str)] = &[
    (
        "packages/codexy-runtime/src/validation/roles_yaml.rs",
        "src/validation/roles_yaml.rs",
    ),
    (
        "packages/codexy-runtime/tests/validator_prompt_metadata.rs",
        "tests/validator_prompt_metadata.rs",
    ),
    (
        "packages/codexy-runtime/src/validation/manifest.rs",
        "src/validation/manifest.rs",
    ),
    (
        "packages/codexy-runtime/src/validation/markdown.rs",
        "src/validation/markdown.rs",
    ),
    (
        "packages/codexy-runtime/src/validation/mcp.rs",
        "src/validation/mcp.rs",
    ),
    (
        "packages/codexy-runtime/src/validation/lsp.rs",
        "src/validation/lsp.rs",
    ),
    (
        "packages/codexy-runtime/tests/architecture_docs_inventory.rs",
        "tests/architecture_docs_inventory.rs",
    ),
    (
        "packages/codexy-runtime/tests/skill_boundary_taxonomy.rs",
        "tests/skill_boundary_taxonomy.rs",
    ),
];
const BOUNDARY_PATHS: &[(&str, &str)] = &[
    (
        "packages/codexy-runtime/src/codegraph/**",
        "src/codegraph/**",
    ),
    ("packages/codexy-runtime/src/lsp/**", "src/lsp/**"),
    ("packages/codexy-runtime/src/mcp.rs", "src/mcp.rs"),
    ("packages/codexy-runtime/src/bin/**", "src/bin/**"),
    ("packages/codexy-runtime/src/version/**", "src/version/**"),
    (
        "packages/codexy-runtime/src/validation/**",
        "src/validation/**",
    ),
    ("packages/codexy-runtime/tests/**", "tests/**"),
];

#[test]
fn documentation_uses_only_module_owned_rust_paths() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let architecture = std::fs::read_to_string(root.join("docs/architecture.md"))?;
    let boundary = std::fs::read_to_string(root.join("docs/plugin-product-boundary.md"))?;

    validate(&architecture, ARCHITECTURE_PATHS)?;
    validate(&boundary, BOUNDARY_PATHS)?;
    assert_rejects_each_stale_path(&architecture, ARCHITECTURE_PATHS)?;
    assert_rejects_each_stale_path(&boundary, BOUNDARY_PATHS)?;
    Ok(())
}

fn validate(
    documentation: &str,
    paths: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    for (module_path, root_path) in paths {
        let documented_module_path = format!("`{module_path}`");
        let documented_root_path = format!("`{root_path}`");
        if !documentation.contains(&documented_module_path) {
            return Err(format!("documentation misses {module_path}").into());
        }
        if documentation.contains(&documented_root_path) {
            return Err(format!("documentation retains stale root path {root_path}").into());
        }
    }
    Ok(())
}

fn assert_rejects_each_stale_path(
    documentation: &str,
    paths: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    for (module_path, root_path) in paths {
        let stale = documentation.replacen(module_path, root_path, 1);
        if validate(&stale, paths).is_ok() {
            return Err(format!("stale root path was accepted: {root_path}").into());
        }
    }
    Ok(())
}
