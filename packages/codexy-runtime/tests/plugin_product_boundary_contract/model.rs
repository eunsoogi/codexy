use serde::Deserialize;

pub(super) const PRODUCTS: [(&str, &str, &str, &[&str], &[&str]); 3] = [
    (
        "codexy",
        "Codexy",
        "plugins/codexy",
        &[],
        &["codexy-github", "codexy-devtools"],
    ),
    (
        "codexy-github",
        "Codexy GitHub",
        "plugins/codexy-github",
        &["codexy"],
        &["codexy-devtools"],
    ),
    (
        "codexy-devtools",
        "Codexy Devtools",
        "plugins/codexy-devtools",
        &["codexy"],
        &["codexy-github"],
    ),
];
pub(super) const TARGETS: [&str; 5] = [
    "codexy",
    "codexy-github",
    "codexy-devtools",
    "repository-only",
    "remove",
];
pub(super) const DISPOSITIONS: [&str; 5] = ["retain", "move", "merge", "split", "remove"];
pub(super) const CATEGORIES: [&str; 10] = [
    "hooks",
    "skills",
    "agents",
    "mcp",
    "lsp",
    "assets",
    "validators",
    "workflows",
    "packaging",
    "public-entrypoints",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BoundaryContract {
    pub(super) schema: String,
    pub(super) products: Vec<Product>,
    pub(super) repository_topology: Topology,
    pub(super) surface_records: Vec<SurfaceRecord>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Product {
    pub(super) id: String,
    pub(super) public_name: String,
    pub(super) package_root: String,
    pub(super) responsibility: String,
    pub(super) depends_on: Vec<String>,
    pub(super) forbidden_dependencies: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Topology {
    pub(super) repository_root: String,
    pub(super) current_rust_runtime_root: String,
    pub(super) future_rust_runtime_root: String,
    pub(super) future_python_distribution_root: String,
    pub(super) root_cargo_workspace: String,
    pub(super) physical_migration: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SurfaceRecord {
    pub(super) id: String,
    pub(super) category: String,
    pub(super) sources: Vec<String>,
    pub(super) target: String,
    pub(super) disposition: String,
}
