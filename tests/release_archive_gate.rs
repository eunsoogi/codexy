use std::process::Command;

use tempfile::tempdir;

#[path = "support/release_archive.rs"]
mod release_archive_support;
use release_archive_support::{copy_tree, make_executable};

fn run_gate(archive: &std::path::Path, plugin_root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_MANIFEST_DIR").to_owned() + "/scripts/inspect-release-archive")
        .arg(archive)
        .arg(plugin_root)
        .output()
        .expect("archive gate should start")
}

fn create_archive(root: &std::path::Path, archive: &std::path::Path) {
    let status = Command::new("tar")
        .args(["-C"])
        .arg(root)
        .args(["-czf"])
        .arg(archive)
        .arg("plugins/codexy")
        .status()
        .expect("tar should start");
    assert!(status.success(), "tar failed: {status}");
}

#[test]
fn archive_gate_allows_documentation_path_examples() {
    let script = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive"),
    )
    .expect("archive gate script");
    assert!(script.contains("--exclude='*.bin'"));
    assert!(script.contains("--exclude='*.md'"));
    assert!(script.contains("--exclude='*.txt'"));
    assert!(script.contains("command -v python3"));
    assert!(script.contains("rg or grep is required"));
    assert!(script.contains("hygiene scan failed"));
    assert!(script.contains("duplicate archive entries"));
    assert!(script.contains("unexpected runtime artifact"));
    assert!(script.contains("invalid JSON-RPC version for response id"));
    assert!(script.contains("unsafe archive path"));
    assert!(script.contains("set(responses) != {1, 2}"));
}

#[test]
fn archive_gate_workflow_covers_every_packaged_surface_and_native_smoke() {
    let workflow = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("runtime workflow");
    assert_eq!(workflow.matches("plugins/codexy/**").count(), 2);
    assert!(workflow.contains("Smoke test native MCP runtimes"));
    assert!(workflow.contains("codexy-mcp-${server}-${PLATFORM}.bin"));
    assert!(workflow.contains("type(message.get(\"id\")) is int"));
}

#[test]
fn archive_gate_accepts_a_complete_valid_package_and_scans_text_files() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    copy_tree(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )
    .expect("copy plugin fixture");
    let runtime = plugin_root.join("runtime");
    std::fs::create_dir_all(&runtime).expect("runtime directory");
    let build = Command::new("cargo")
        .args([
            "build",
            "--offline",
            "--release",
            "--bin",
            "codexy-mcp-lsp",
            "--bin",
            "codexy-mcp-codegraph",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("release runtime build");
    assert!(build.success(), "release runtime build failed");
    let host_platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("linux", "x86_64") => "linux-x86_64",
        _ => panic!("unsupported test host platform"),
    };
    for (server, binary) in [
        ("lsp", "codexy-mcp-lsp"),
        ("codegraph", "codexy-mcp-codegraph"),
    ] {
        for platform in ["darwin-arm64", "linux-x86_64"] {
            let path = runtime.join(format!("codexy-mcp-{server}-{platform}.bin"));
            if platform == host_platform {
                std::fs::copy(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("target/release")
                        .join(binary),
                    &path,
                )
                .expect("copy runtime");
            } else {
                let header = if platform == "darwin-arm64" {
                    vec![0xcf, 0xfa, 0xed, 0xfe]
                } else {
                    vec![0x7f, b'E', b'L', b'F']
                };
                std::fs::write(&path, header.repeat(1024)).expect("runtime fixture");
            }
            make_executable(&path).expect("runtime permissions");
        }
    }
    let archive = root.path().join("valid.tar.gz");
    create_archive(root.path(), &archive);
    let output = run_gate(&archive, &plugin_root);
    assert!(
        output.status.success(),
        "valid fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::write(runtime.join("debug.log"), "debug\n").expect("runtime extra fixture");
    let extra_runtime_archive = root.path().join("extra-runtime.tar.gz");
    create_archive(root.path(), &extra_runtime_archive);
    assert!(
        !run_gate(&extra_runtime_archive, &plugin_root)
            .status
            .success()
    );
    std::fs::remove_file(runtime.join("debug.log")).expect("remove runtime extra");

    let missing_runtime = runtime.join("codexy-mcp-lsp-linux-x86_64.bin");
    let missing_bytes = std::fs::read(&missing_runtime).expect("read runtime");
    std::fs::remove_file(&missing_runtime).expect("remove runtime");
    let missing_archive = root.path().join("missing-runtime.tar.gz");
    create_archive(root.path(), &missing_archive);
    assert!(!run_gate(&missing_archive, &plugin_root).status.success());
    std::fs::write(&missing_runtime, missing_bytes).expect("restore runtime");

    let malformed_bytes = std::fs::read(&missing_runtime).expect("read runtime again");
    std::fs::write(&missing_runtime, b"not-a-binary").expect("malform runtime");
    let malformed_archive = root.path().join("malformed-runtime.tar.gz");
    create_archive(root.path(), &malformed_archive);
    assert!(!run_gate(&malformed_archive, &plugin_root).status.success());
    std::fs::write(&missing_runtime, malformed_bytes).expect("restore valid runtime");

    std::fs::write(plugin_root.join("README.md"), "AKIA1234567890ABCDEF\n")
        .expect("secret fixture");
    let secret_archive = root.path().join("secret.tar.gz");
    create_archive(root.path(), &secret_archive);
    assert!(!run_gate(&secret_archive, &plugin_root).status.success());
    std::fs::remove_file(plugin_root.join("README.md")).expect("remove visible secret");

    std::fs::write(plugin_root.join(".rgignore"), "hidden-secret.txt\n").expect("ignore fixture");
    std::fs::write(
        plugin_root.join("hidden-secret.txt"),
        "AKIA1234567890ABCDEF\n",
    )
    .expect("ignored secret fixture");
    let ignored_secret_archive = root.path().join("ignored-secret.tar.gz");
    create_archive(root.path(), &ignored_secret_archive);
    assert!(
        !run_gate(&ignored_secret_archive, &plugin_root)
            .status
            .success()
    );
}

#[test]
fn archive_gate_rejects_incomplete_packaged_surface() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    std::fs::write(plugin_root.join("plugin.txt"), "incomplete\n").expect("fixture");
    let archive = root.path().join("incomplete.tar.gz");
    create_archive(root.path(), &archive);

    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
}

#[test]
fn archive_gate_rejects_traversal_member_before_extraction() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    let archive = root.path().join("traversal.tar.gz");
    let script = r#"import io, sys, tarfile
with tarfile.open(sys.argv[1], "w:gz") as archive:
    info = tarfile.TarInfo("plugins/codexy/../escape")
    payload = b"escape"
    info.size = len(payload)
    archive.addfile(info, io.BytesIO(payload))
"#;
    let status = Command::new("python3")
        .args(["-c", script, archive.to_str().expect("archive path")])
        .status()
        .expect("python should start");
    assert!(status.success(), "traversal archive fixture failed");
    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe archive path"));
}

#[test]
fn archive_gate_rejects_unexpected_file_and_stale_content() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    let file = plugin_root.join("plugin.txt");
    std::fs::write(&file, "version=1.1.0\n").expect("fixture");
    let archive = root.path().join("stale.tar.gz");
    create_archive(root.path(), &archive);
    std::fs::write(&file, "version=1.1.1\n").expect("stale fixture");
    assert!(!run_gate(&archive, &plugin_root).status.success());

    let extra_root = tempdir().expect("extra tempdir");
    let extra_plugin = extra_root.path().join("plugins/codexy");
    std::fs::create_dir_all(&extra_plugin).expect("plugin directory");
    std::fs::write(extra_plugin.join("plugin.txt"), "version=1.1.0\n").expect("fixture");
    std::fs::write(extra_plugin.join("unexpected.txt"), "extra\n").expect("fixture");
    let extra_archive = extra_root.path().join("extra.tar.gz");
    create_archive(extra_root.path(), &extra_archive);
    std::fs::remove_file(extra_plugin.join("unexpected.txt")).expect("stage mutation");
    assert!(!run_gate(&extra_archive, &extra_plugin).status.success());
}

#[cfg(unix)]
#[test]
fn archive_gate_rejects_symlink_entries() {
    use std::os::unix::fs::symlink;

    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    symlink("/etc/passwd", plugin_root.join("bad-link")).expect("symlink fixture");
    let archive = root.path().join("symlink.tar.gz");
    create_archive(root.path(), &archive);

    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
}
