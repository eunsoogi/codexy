use crate::support::release_archive as release_archive_support;

use release_archive_support::assert_runtime_workflow_contract;

#[test]
fn archive_gate_workflow_covers_every_packaged_surface_and_native_smoke() {
    let (workflow, archive_inspector) = workflow_sources();
    assert_eq!(workflow.matches("plugins/codexy/**").count(), 2);
    assert_runtime_workflow_contract(&workflow, &archive_inspector);
}

#[test]
fn archive_gate_workflow_rejects_duplicate_branches_and_suffixed_runtime_paths() {
    let (workflow, archive_inspector) = workflow_sources();
    let decoy = "case \"$state\" in\n          candidate-proven)\n            scripts/materialize-runtime-release-archive dist/selected.tar.gz dist/codexy-marketplace-plugin.tar.gz\n            scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz final-inspect/plugins/codexy public-release\n            ;;";
    for invalid in [
        workflow.replacen("case \"$state\" in", decoy, 1),
        workflow.replace(
            "plugins/codexy/runtime/codexy-mcp-lsp-darwin-arm64.bin",
            "plugins/codexy/runtime/codexy-mcp-lsp-darwin-arm64.bin-extra",
        ),
    ] {
        assert!(
            std::panic::catch_unwind(|| {
                assert_runtime_workflow_contract(&invalid, &archive_inspector)
            })
            .is_err()
        );
    }
}

fn workflow_sources() -> (String, String) {
    let workflow = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("runtime workflow");
    let archive_inspector = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive"),
    )
    .expect("archive inspector");
    (workflow, archive_inspector)
}

#[test]
fn archive_fixture_uses_cargo_provided_runtime_binaries_without_nested_builds() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/release_archive.rs"),
    )
    .expect("archive fixture support source");
    assert!(
        source
            .lines()
            .all(|line| line.trim() != "let build = Command::new(\"cargo\")"),
        "archive fixtures must not invoke a nested Cargo build"
    );
    for binary in [
        "CARGO_BIN_EXE_codexy-mcp-lsp",
        "CARGO_BIN_EXE_codexy-mcp-codegraph",
    ] {
        assert!(
            source
                .lines()
                .any(|line| line.split('"').any(|token| token == binary)),
            "archive fixture must use Cargo-provided runtime {binary}"
        );
    }
}
