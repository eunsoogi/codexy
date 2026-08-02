use std::{fs, path::Path};

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

#[test]
fn namespace_diagnostic_selector_is_reachable() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR")).is_dir());
}

#[test]
fn namespace_diagnostic_is_bounded_to_existing_suite_all_namespaces() {
    let script = read("scripts/profile_rust_namespace_diagnostic.py");
    for namespace in [
        "agent", "child_a", "child_b", "hook", "loc", "policy", "system", "workflow",
    ] {
        assert!(script.contains(&format!("\"{namespace}\"")));
    }
    assert!(script.contains("suite_all"));
    assert!(script.contains("suite_archive"));
    assert!(script.contains("tolerance"));
    assert!(script.contains("source_digest"));
    assert!(script.contains("static_owner_mapping"));
}

#[test]
fn namespace_diagnostic_workflow_is_manual_windows_only() {
    let workflow = read(".github/workflows/rust-namespace-diagnostic.yml");
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("runs-on: windows-latest"));
    assert!(workflow.contains("profile_rust_namespace_diagnostic.py"));
    assert!(!workflow.contains("pull_request:"));
    assert!(!workflow.contains("push:"));
}
