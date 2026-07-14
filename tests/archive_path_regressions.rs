use std::process::Command;

use tempfile::tempdir;

fn run_gate(archive: &std::path::Path, plugin_root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_MANIFEST_DIR").to_owned() + "/scripts/inspect-release-archive")
        .args([archive.to_str().unwrap(), plugin_root.to_str().unwrap()])
        .output()
        .expect("archive gate should start")
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
