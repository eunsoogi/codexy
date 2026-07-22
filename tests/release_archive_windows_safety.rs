use std::process::Command;

use tempfile::tempdir;

fn run_gate(archive: &std::path::Path, plugin_root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_MANIFEST_DIR").to_owned() + "/scripts/inspect-release-archive")
        .arg(archive)
        .arg(plugin_root)
        .output()
        .expect("archive gate should start")
}

fn create_archive(archive: &std::path::Path, scenario: &str) {
    let script = r#"import io, sys, tarfile
archive_path, scenario = sys.argv[1:]
with tarfile.open(archive_path, "w:gz") as archive:
    names = {
        "backslash": [r"plugins/codexy/C:\escape"],
        "casefold": ["plugins/codexy/MCP/Server.exe", "plugins/codexy/mcp/server.exe"],
        "control": ["plugins/codexy/bad\nname"],
        "device": ["plugins/codexy/NUL.txt"],
        "drive-relative": ["plugins/codexy/C:escape"],
        "trailing": ["plugins/codexy/bad. "],
        "total-size": ["plugins/codexy/one", "plugins/codexy/two"],
    }
    if scenario == "hardlink":
        info = tarfile.TarInfo("plugins/codexy/hardlink")
        info.type = tarfile.LNKTYPE
        info.linkname = "plugins/codexy/target"
        archive.addfile(info)
    else:
        for name in names[scenario]:
            payload = b"fixture"
            info = tarfile.TarInfo(name)
            info.size = len(payload)
            archive.addfile(info, io.BytesIO(payload))
"#;
    let status = Command::new("python3")
        .args([
            "-c",
            script,
            archive.to_str().expect("archive path"),
            scenario,
        ])
        .status()
        .expect("python should start");
    assert!(status.success(), "archive fixture failed for {scenario}");
}

#[test]
fn archive_gate_rejects_windows_aliases_collisions_and_hardlinks_before_extraction() {
    for (scenario, expected) in [
        ("backslash", "unsafe archive path"),
        ("casefold", "case-folding archive entries collide"),
        ("control", "unsafe archive path"),
        ("device", "unsafe Windows device archive path"),
        ("drive-relative", "unsafe archive path"),
        ("trailing", "unsafe Windows-normalized archive path"),
        ("hardlink", "unsupported archive entry type"),
    ] {
        let root = tempdir().expect("tempdir");
        let plugin_root = root.path().join("plugins/codexy");
        std::fs::create_dir_all(&plugin_root).expect("plugin fixture root");
        let archive = root.path().join(format!("{scenario}.tar.gz"));
        create_archive(&archive, scenario);

        let output = run_gate(&archive, &plugin_root);
        assert!(!output.status.success(), "{scenario} unexpectedly passed");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{scenario} failed for the wrong reason: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn archive_gate_rejects_cumulative_uncompressed_size_before_extraction() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin fixture root");
    let archive = root.path().join("total-size.tar.gz");
    create_archive(&archive, "total-size");

    let output =
        Command::new(env!("CARGO_MANIFEST_DIR").to_owned() + "/scripts/inspect-release-archive")
            .arg(&archive)
            .arg(&plugin_root)
            .env("MAX_ARCHIVE_TOTAL_BYTES", "10")
            .output()
            .expect("archive gate should start");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("archive uncompressed size exceeds the configured limit")
    );
}
