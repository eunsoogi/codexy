use std::process::Command;

use tempfile::tempdir;

fn run_gate(archive: &std::path::Path, plugin_root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_MANIFEST_DIR").to_owned() + "/scripts/inspect-release-archive")
        .args([archive.to_str().unwrap(), plugin_root.to_str().unwrap()])
        .output()
        .expect("archive gate should start")
}

fn write_single_member_archive(archive: &std::path::Path, entry: &str) {
    let script = r#"import io, sys, tarfile
with tarfile.open(sys.argv[1], "w:gz") as archive:
    info = tarfile.TarInfo(sys.argv[2])
    payload = b"fixture"
    info.size = len(payload)
    archive.addfile(info, io.BytesIO(payload))
"#;
    assert!(
        Command::new("python3")
            .args(["-c", script, archive.to_str().unwrap(), entry])
            .status()
            .expect("python should start")
            .success()
    );
}

#[test]
fn rejects_canonical_duplicate_members() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    let archive = root.path().join("duplicate.tar.gz");
    let script = r#"import io, sys, tarfile
with tarfile.open(sys.argv[1], "w:gz") as archive:
    for name in ("plugins/codexy/value", "plugins/codexy/./value"):
        info = tarfile.TarInfo(name)
        payload = b"same"
        info.size = len(payload)
        archive.addfile(info, io.BytesIO(payload))
"#;
    assert!(
        Command::new("python3")
            .args(["-c", script, archive.to_str().unwrap()])
            .status()
            .expect("python should start")
            .success()
    );
    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate archive entries"));
}

#[test]
fn rejects_embedded_double_slash_member() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    let archive = root.path().join("double-slash.tar.gz");
    let script = r#"import io, sys, tarfile
with tarfile.open(sys.argv[1], "w:gz") as archive:
    info = tarfile.TarInfo("plugins/codexy//hooks/foo")
    payload = b"unsafe"
    info.size = len(payload)
    archive.addfile(info, io.BytesIO(payload))
"#;
    assert!(
        Command::new("python3")
            .args(["-c", script, archive.to_str().unwrap()])
            .status()
            .expect("python should start")
            .success()
    );
    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe archive path"));
}

#[test]
fn rejects_repeated_dot_segment_alias_without_rejecting_canonical_member() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");

    let canonical_archive = root.path().join("canonical.tar.gz");
    write_single_member_archive(
        &canonical_archive,
        "plugins/codexy/.codex-plugin/plugin.json",
    );
    let canonical_output = run_gate(&canonical_archive, &plugin_root);
    assert!(
        !canonical_output.status.success(),
        "fixture is intentionally incomplete"
    );
    assert!(
        !String::from_utf8_lossy(&canonical_output.stderr).contains("unsafe archive path"),
        "canonical member was rejected as unsafe: {}",
        String::from_utf8_lossy(&canonical_output.stderr)
    );

    let alias_archive = root.path().join("repeated-dot-segment.tar.gz");
    write_single_member_archive(
        &alias_archive,
        "plugins/codexy/././.codex-plugin/plugin.json",
    );
    let alias_output = run_gate(&alias_archive, &plugin_root);
    assert!(!alias_output.status.success());
    assert!(String::from_utf8_lossy(&alias_output.stderr).contains("unsafe archive path"));
}
