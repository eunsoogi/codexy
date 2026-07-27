use crate::support::{FixtureCommand as Command, materialize_lf_text_fixture, fixture_path_text};

use serde_json::Value;

use crate::support::release_archive as release_archive_support;
use release_archive_support::{complete_plugin_fixture, create_archive};

#[test]
fn inventory_metadata_local_paths_fail_with_both_archive_scanners() {
    let root = tempfile::tempdir().expect("tempdir");
    let plugin_root = complete_plugin_fixture(root.path()).expect("complete fixture");
    let inventory_path = plugin_root.join("hooks/policy-inventory.json");
    let mut inventory: Value = serde_json::from_slice(&std::fs::read(&inventory_path).expect("inventory")).expect("JSON");
    inventory["rules"][0]["evidence"].as_array_mut().expect("evidence array").push(Value::from("/home/alice/private-state"));
    std::fs::write(&inventory_path, serde_json::to_vec(&inventory).expect("JSON")).expect("inventory fixture");
    let archive = root.path().join("evidence.tar.gz");
    create_archive(root.path(), &archive).expect("archive fixture");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive");
    let canonical_source = root.path().join("inspect-release-archive");
    materialize_lf_text_fixture(&source, &canonical_source).expect("canonical archive inspector");
    for grep_backend in [false, true] {
        let mut command = Command::new(if grep_backend { "sh" } else { canonical_source.to_str().expect("gate path") });
        if grep_backend {
            let script_dir = fixture_path_text(source.parent().expect("scripts directory")).expect("fixture script directory");
            let script = std::fs::read_to_string(&canonical_source).expect("gate script").replacen("if command -v rg >/dev/null 2>&1; then", "if false; then", 1).replacen("script_dir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)", &format!("script_dir={script_dir}"), 1);
            command.arg("-c").arg(script).arg("inventory-grep");
        }
        let output = command.arg_path(&archive).arg_path(&plugin_root).output().expect("archive gate should start");
        assert!(!output.status.success(), "archive scanner accepted metadata, grep={grep_backend}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("archive contains a secret or local path"),
            "archive scanner diagnostic missing, grep={grep_backend}, status={}, stdout={:?}, stderr={:?}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
