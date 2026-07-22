#[path = "support/release_archive.rs"]
mod release_archive_support;

use release_archive_support::assert_runtime_workflow_contract;

#[test]
fn archive_gate_workflow_covers_every_packaged_surface_and_native_smoke() {
    let workflow = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("runtime workflow");
    assert_eq!(workflow.matches("plugins/codexy/**").count(), 2);
    assert_runtime_workflow_contract(&workflow);
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
