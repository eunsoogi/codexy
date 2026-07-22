use std::process::Command;

use serde_json::Value;

#[path = "support/release_archive.rs"]
mod release_archive_support;
use release_archive_support::{complete_plugin_fixture, create_archive};

fn run_gate(
    archive: &std::path::Path,
    plugin_root: &std::path::Path,
    grep_backend: bool,
) -> std::process::Output {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive");
    let mut command = Command::new(if grep_backend {
        "sh"
    } else {
        source.to_str().expect("gate path")
    });
    if grep_backend {
        let script = std::fs::read_to_string(&source)
            .expect("gate script")
            .replacen(
                "if command -v rg >/dev/null 2>&1; then",
                "if false; then",
                1,
            )
            .replacen(
                "script_dir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)",
                &format!(
                    "script_dir={}",
                    source.parent().expect("scripts directory").display()
                ),
                1,
            );
        command.arg("-c").arg(script).arg("inventory-grep");
    }
    command
        .args([archive, plugin_root])
        .output()
        .expect("archive gate should start")
}

#[test]
fn policy_inventory_metadata_local_paths_fail_with_both_scanners() {
    for field in [
        "evidence",
        "rationale",
        "unavailableEvent",
        "unavailableInput",
    ] {
        for grep_backend in [false, true] {
            let root = tempfile::tempdir().expect("tempdir");
            let plugin_root = complete_plugin_fixture(root.path()).expect("complete fixture");
            let inventory_path = plugin_root.join("hooks/policy-inventory.json");
            let mut inventory: Value =
                serde_json::from_slice(&std::fs::read(&inventory_path).expect("inventory"))
                    .expect("JSON");
            let rule = &mut inventory["rules"][0];
            if field == "evidence" {
                rule[field]
                    .as_array_mut()
                    .expect("evidence array")
                    .push(Value::from("/home/alice/private-state"));
            } else {
                rule[field] = Value::from("proof at /home/alice/private-state");
            }
            std::fs::write(
                &inventory_path,
                serde_json::to_vec(&inventory).expect("JSON"),
            )
            .expect("inventory fixture");
            let archive = root.path().join(format!("{field}-{grep_backend}.tar.gz"));
            create_archive(root.path(), &archive).expect("archive fixture");
            let output = run_gate(&archive, &plugin_root, grep_backend);
            assert!(
                !output.status.success(),
                "{field} escaped scanner backend grep={grep_backend}"
            );
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("archive contains a secret or local path"),
                "unexpected failure for {field}, grep={grep_backend}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
