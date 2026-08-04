use crate::support::{FixtureCommand as Command, materialize_lf_text_fixture};

use serde_json::Value;

use crate::support::release_archive as release_archive_support;
use release_archive_support::{complete_plugin_fixture, create_archive};

#[test]
fn inventory_metadata_local_paths_fail_archive_inspector() {
    let root = tempfile::tempdir().expect("tempdir");
    let plugin_root = complete_plugin_fixture(root.path()).expect("complete fixture");
    let inventory_path = plugin_root.join("hooks/policy-inventory.json");
    let mut inventory: Value = serde_json::from_slice(&std::fs::read(&inventory_path).expect("inventory")).expect("JSON");
    inventory["rules"][0]["evidence"].as_array_mut().expect("evidence array").push(Value::from("/home/alice/private-state"));
    std::fs::write(&inventory_path, serde_json::to_vec(&inventory).expect("JSON")).expect("inventory fixture");
    let archive = root.path().join("evidence.tar.gz");
    create_archive(root.path(), &archive).expect("archive fixture");
    std::fs::write(plugin_root.join("hooks/hooks.json"), b"{}").expect("co-invalid staged fixture");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive");
    let canonical_source = root.path().join("inspect-release-archive");
    materialize_lf_text_fixture(&source, &canonical_source).expect("canonical archive inspector");
    let mut command = Command::new(&canonical_source);
    let output = command.arg_path(&archive).arg_path(&plugin_root).output().expect("archive gate should start");
    assert!(!output.status.success(), "archive scanner accepted metadata");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("archive contains a secret or local path"),
        "archive scanner diagnostic missing, status={}, stdout={:?}, stderr={:?}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
