use crate::support::release_archive as release_archive_support;

#[test]
fn archive_gate_workflow_covers_every_packaged_surface_and_native_smoke() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))
            .expect("runtime workflow");
    let inspector = std::fs::read_to_string(root.join("scripts/inspect-release-archive"))
        .expect("archive inspector");
    let trigger_paths = workflow
        .split("permissions:")
        .next()
        .expect("workflow trigger paths");
    assert_eq!(trigger_paths.matches("plugins/codexy/**").count(), 2);
    for path in [
        "scripts/inspect-release-archive",
        "scripts/check-release-archive-content",
    ] {
        assert_eq!(
            trigger_paths.matches(path).count(),
            2,
            "workflow must trigger for {path}"
        );
    }
    release_archive_support::assert_runtime_workflow_contract(&workflow, &inspector);
}
