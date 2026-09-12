use std::path::Path;

use crate::support::FixtureCommand;

pub(super) fn copy_selected_sources(repo_root: &Path) {
    std::fs::create_dir_all(repo_root.join("packages/codexy-runtime/src/version"))
        .expect("candidate version parent");
    std::fs::create_dir_all(repo_root.join("plugins/codexy-devtools/mcp"))
        .expect("candidate devtools parent");
    for relative in [
        ".github/workflows/python-package.yml",
        "packages/codexy-runtime/src/version/bootstrap.rs",
        "plugins/codexy-devtools/mcp/codexy-mcp-devtools",
        "plugins/codexy-devtools/runtime-release.json",
        "scripts/verify_public_marketplace_bundle.py",
        "scripts/public_marketplace_bundle_support.py",
    ] {
        let source = codexy_runtime::paths::repository_root().join(relative);
        let target = repo_root.join(relative);
        std::fs::copy(source, target).expect("candidate selected-release source");
    }
}

pub(super) fn run_source_projection(plugin_root: &Path) -> std::process::Output {
    let mut command = FixtureCommand::new("python3");
    command
        .arg(
            codexy_runtime::paths::repository_root()
                .join("scripts/inspect-release-archive-contract.py"),
        )
        .arg("source-projection")
        .arg_path(plugin_root)
        .output()
        .expect("source projection should start")
}
