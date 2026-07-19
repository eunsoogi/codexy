use std::process::Command;

use tempfile::tempdir;

#[path = "support/release_archive.rs"]
mod release_archive_support;
use release_archive_support::{complete_plugin_fixture_with_stubbed_runtime, create_archive};

fn assert_private_key_rejected(name: &str, pem: &str) {
    let root = tempdir().expect("tempdir");
    let plugin_root =
        complete_plugin_fixture_with_stubbed_runtime(root.path()).expect("complete plugin fixture");
    std::fs::write(plugin_root.join(name), pem).expect("private-key fixture");
    let archive = root.path().join(format!("{name}.tar.gz"));
    create_archive(root.path(), &archive).expect("archive fixture");
    let output =
        Command::new(env!("CARGO_MANIFEST_DIR").to_owned() + "/scripts/inspect-release-archive")
            .arg(&archive)
            .arg(&plugin_root)
            .output()
            .expect("archive gate should start");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("archive contains a secret or local path")
    );
}

#[test]
fn archive_gate_rejects_a_generic_private_key() {
    assert_private_key_rejected("generic-private-key.pem", "-----BEGIN PRIVATE KEY-----\n");
}

#[test]
fn archive_gate_rejects_an_encrypted_private_key() {
    assert_private_key_rejected(
        "encrypted-private-key.pem",
        "-----BEGIN ENCRYPTED PRIVATE KEY-----\n",
    );
}
