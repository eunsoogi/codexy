use std::process::Command;

use tempfile::tempdir;

#[path = "support/release_archive.rs"]
mod release_archive_support;
use release_archive_support::{
    assert_structured_literals, complete_plugin_fixture, create_archive,
};

fn run_gate_at(
    gate: &std::path::Path,
    archive: &std::path::Path,
    plugin_root: &std::path::Path,
) -> std::process::Output {
    Command::new(gate)
        .arg(archive)
        .arg(plugin_root)
        .output()
        .expect("archive gate should start")
}

#[cfg(unix)]
fn grep_backend_gate(root: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive");
    let source_dir = source.parent().expect("script directory");
    let script = std::fs::read_to_string(&source).expect("archive gate script");
    let script = script.replacen(
        "script_dir=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)",
        &format!("script_dir={}", source_dir.display()),
        1,
    );
    let script = script.replacen(
        "if command -v rg >/dev/null 2>&1; then",
        "if false; then",
        1,
    );
    assert_structured_literals(&script, "grep backend selection", &["if false; then"]);
    let gate = root.join("inspect-release-archive-grep");
    std::fs::write(&gate, script).expect("grep backend gate");
    let mut permissions = std::fs::metadata(&gate)
        .expect("gate metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&gate, permissions).expect("gate executable");
    gate
}

#[cfg(unix)]
#[test]
fn archive_gate_scans_nul_prefixed_binary_assets_in_both_backends() {
    let root = tempdir().expect("tempdir");
    let plugin_root = complete_plugin_fixture(root.path()).expect("complete plugin fixture");
    let asset = plugin_root.join("assets/binary-asset.png");
    let source_gate =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive");
    let grep_gate = grep_backend_gate(root.path());

    std::fs::write(&asset, b"\0safe binary asset\n").expect("safe binary fixture");
    let valid_archive = root.path().join("valid-binary-asset.tar.gz");
    create_archive(root.path(), &valid_archive).expect("archive fixture");
    for (backend, gate) in [("rg", &source_gate), ("grep", &grep_gate)] {
        let output = run_gate_at(gate, &valid_archive, &plugin_root);
        assert!(
            output.status.success(),
            "{backend} valid fixture failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let marker = "AKIA1234567890ABCDEF";
    std::fs::write(&asset, format!("\0{marker}\n")).expect("NUL-prefixed marker fixture");
    let marker_archive = root.path().join("nul-prefixed-marker.tar.gz");
    create_archive(root.path(), &marker_archive).expect("archive fixture");

    for gate in [&source_gate, &grep_gate] {
        let output = run_gate_at(gate, &marker_archive, &plugin_root);
        assert!(!output.status.success(), "NUL-prefixed marker must fail");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("archive contains a secret or local path"));
        assert!(!stdout.contains(marker));
        assert!(!stderr.contains(marker));
    }
}
