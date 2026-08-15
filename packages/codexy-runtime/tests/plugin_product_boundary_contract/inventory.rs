use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) fn record_matrix() -> BTreeMap<&'static str, (&'static str, &'static str)> {
    BTreeMap::from([
        ("hooks.instruction", ("codexy", "retain")),
        ("hooks.github", ("codexy-github", "move")),
        ("hooks.policy-core", ("codexy", "retain")),
        ("skills.core", ("codexy", "retain")),
        ("skills.github", ("codexy-github", "move")),
        ("skills.devtools", ("codexy-devtools", "retain")),
        ("skills.repository", ("repository-only", "move")),
        ("agents.specialists", ("codexy", "retain")),
        ("agents.github", ("codexy-github", "move")),
        ("agents.devtools", ("codexy-devtools", "retain")),
        ("mcp.codegraph", ("codexy-devtools", "move")),
        ("mcp.lsp", ("codexy-devtools", "move")),
        ("mcp.runtimes", ("codexy-devtools", "move")),
        ("lsp.integration", ("codexy-devtools", "move")),
        ("runtime.codegraph", ("codexy-devtools", "move")),
        ("runtime.lsp", ("codexy-devtools", "move")),
        ("runtime.entrypoints", ("codexy-devtools", "move")),
        ("runtime.repository", ("repository-only", "split")),
        ("runtime.release", ("codexy-devtools", "retain")),
        ("assets.repository", ("repository-only", "retain")),
        ("assets.plugin", ("codexy", "retain")),
        ("repository.governance", ("repository-only", "retain")),
        ("repository.workflows", ("repository-only", "retain")),
        ("repository.packaging", ("repository-only", "split")),
        ("public.core", ("codexy", "retain")),
        ("public.devtools", ("codexy-devtools", "retain")),
        ("public.repository", ("repository-only", "retain")),
    ])
}
pub(super) fn governed_universe(
    root: &Path,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let mut governed = BTreeSet::new();
    for path in [
        ".agents/skills",
        ".codex",
        "plugins/codexy/hooks",
        "plugins/codexy/skills",
        "plugins/codexy/agents",
        "plugins/codexy-devtools/mcp",
        "plugins/codexy-devtools/lsp",
        "plugins/codexy-devtools/skills",
        "plugins/codexy-devtools/agents",
        "plugins/codexy-github/hooks",
        "plugins/codexy-github/skills",
        "plugins/codexy-github/agents",
        "assets",
        "plugins/codexy/assets",
        "scripts",
        "packages/codexy-runtime/src/validation",
        "packages/codexy-runtime/tests",
        ".github/workflows",
        "packages/codexy-runtime/src/codegraph",
        "packages/codexy-runtime/src/lsp",
        "packages/codexy-runtime/src/version",
        "packages/codexy-runtime/src/bin",
        "packages/getcodexy",
    ] {
        governed.extend(files(root.join(path))?);
    }
    for path in [
        "packages/codexy-runtime/src/mcp.rs",
        "packages/codexy-runtime/Cargo.toml",
        "packages/codexy-runtime/Cargo.lock",
        "packages/codexy-runtime/rust-toolchain.toml",
        "packages/codexy-runtime/rustfmt.toml",
        "packages/codexy-runtime/clippy.toml",
        "plugins/codexy/bootstrap-codexy-agents",
        "plugins/codexy/check-codexy-agents",
        ".agents/plugins/marketplace.json",
        ".agents/plugins/release-publish-contract.json",
        ".agents/plugins/runtime-activation.json",
        "plugins/codexy-devtools/runtime-release.json",
        "plugins/codexy-devtools/.codex/lsp-client.json",
        "plugins/codexy/.codex-plugin/plugin.json",
        "plugins/codexy/agents/openai.yaml",
        "plugins/codexy-devtools/.codex-plugin/plugin.json",
        "plugins/codexy-devtools/agents/openai.yaml",
        "plugins/codexy-github/.codex-plugin/plugin.json",
        "plugins/codexy-github/agents/openai.yaml",
        "README.md",
        "README.ko.md",
    ] {
        governed.extend(files(root.join(path))?);
    }
    Ok(governed)
}
pub(super) fn files(path: PathBuf) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if path.is_file() {
        return Ok(vec![repository_identity(
            codexy_runtime::paths::repository_root(),
            &path,
        )?]);
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(path)? {
        found.extend(files(entry?.path())?);
    }
    Ok(found)
}
pub(super) fn repository_identity(
    root: &Path,
    path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let relative = path.strip_prefix(root)?;
    let mut components = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::Normal(part) => {
                components.push(part.to_string_lossy().into_owned())
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Err("unsafe repository-relative path".into());
            }
        }
    }
    if components.is_empty() {
        return Err("empty repository-relative path".into());
    }
    Ok(components.join("/"))
}
