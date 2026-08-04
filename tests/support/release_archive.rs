//! Root-owned archive fixture context for the reusable test-support crate.
//!
//! Cargo injects these paths for each integration target. They intentionally
//! stay here, outside the shared package, so archive generation only consumes
//! the caller-provided fixture host.

use std::path::Path;

pub(crate) use crate::support::make_executable;
pub(crate) use codexy_test_support::release_archive::{
    archive_entry, archive_evidence, assert_archive_scanner_contract,
    assert_runtime_workflow_contract, assert_structured_absent_literals,
    assert_structured_literals, create_archive, create_archive_with_commands,
    fixture_host_platform, governed_archive_mode,
};

struct RootFixtureHost;

impl codexy_test_support::FixtureHost for RootFixtureHost {
    fn manifest_dir(&self) -> &Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }
    fn validate_config_binary(&self) -> &Path {
        Path::new(env!("CARGO_BIN_EXE_codexy-validate"))
    }
    fn mcp_lsp_binary(&self) -> &Path {
        Path::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
    }
    fn mcp_codegraph_binary(&self) -> &Path {
        Path::new(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))
    }
    fn make_executable(&self, path: &Path) -> std::io::Result<()> {
        crate::support::make_executable(path)
    }
    fn materialize_admission_runtime_suite(&self, plugin_root: &Path) -> std::io::Result<()> {
        crate::support::materialize_admission_runtime_suite(plugin_root)
    }
}

pub(crate) fn inspect_archive(
    archive: &Path,
    plugin_root: &Path,
    path: Option<&Path>,
) -> std::io::Result<std::process::Output> {
    codexy_test_support::release_archive::inspect_archive(
        &RootFixtureHost,
        archive,
        plugin_root,
        path,
    )
}
pub(crate) fn copy_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    codexy_test_support::release_archive::copy_tree(&RootFixtureHost, source, target)
}
pub(crate) fn complete_plugin_fixture(root: &Path) -> std::io::Result<std::path::PathBuf> {
    codexy_test_support::release_archive::complete_plugin_fixture(&RootFixtureHost, root)
}
pub(crate) fn complete_plugin_fixture_with_stubbed_runtime(
    root: &Path,
) -> std::io::Result<std::path::PathBuf> {
    codexy_test_support::release_archive::complete_plugin_fixture_with_stubbed_runtime(
        &RootFixtureHost,
        root,
    )
}
