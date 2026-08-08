use crate::support::TestResult;

const ARCHITECTURE_PATHS: &[&str] = &[
    "packages/codexy-runtime/src/validation/roles_yaml.rs",
    "packages/codexy-runtime/tests/validator_prompt_metadata.rs",
    "packages/codexy-runtime/src/validation/instruction_policy.rs",
    "packages/codexy-runtime/tests/architecture_docs_inventory.rs",
];
const BOUNDARY_PATHS: &[&str] = &[
    "packages/codexy-runtime/src/codegraph/**",
    "packages/codexy-runtime/src/lsp/**",
    "packages/codexy-runtime/src/mcp.rs",
    "packages/codexy-runtime/src/bin/**",
    "packages/codexy-runtime/src/version/**",
    "packages/codexy-runtime/src/validation/**",
    "packages/codexy-runtime/tests/**",
];
const STALE_ROOT_PATHS: &[&str] = &[
    "`src/codegraph/**`",
    "`src/lsp/**`",
    "`src/mcp.rs`",
    "`src/bin/**`",
    "`src/version/**`",
    "`src/validation/**`",
    "`tests/**`",
    "`tests/validator_prompt_metadata.rs`",
    "`tests/validator_instruction_policy.rs`",
];

#[test]
fn documentation_uses_only_module_owned_rust_paths() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let architecture = std::fs::read_to_string(root.join("docs/architecture.md"))?;
    let boundary = std::fs::read_to_string(root.join("docs/plugin-product-boundary.md"))?;

    validate(&architecture, &boundary)?;
    let stale_architecture = architecture.replacen(
        "packages/codexy-runtime/src/validation/roles_yaml.rs",
        "src/validation/roles_yaml.rs",
        1,
    );
    assert!(validate(&stale_architecture, &boundary).is_err());
    let stale_boundary = boundary.replacen(
        "packages/codexy-runtime/src/codegraph/**",
        "src/codegraph/**",
        1,
    );
    assert!(validate(&architecture, &stale_boundary).is_err());
    Ok(())
}

fn validate(architecture: &str, boundary: &str) -> Result<(), Box<dyn std::error::Error>> {
    for path in ARCHITECTURE_PATHS {
        if !architecture.contains(path) {
            return Err(format!("architecture documentation misses {path}").into());
        }
    }
    for path in BOUNDARY_PATHS {
        if !boundary.contains(path) {
            return Err(format!("product boundary documentation misses {path}").into());
        }
    }
    for path in STALE_ROOT_PATHS {
        if architecture.contains(path) || boundary.contains(path) {
            return Err(format!("documentation retains stale root path {path}").into());
        }
    }
    Ok(())
}
