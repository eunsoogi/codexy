use std::process::Command;

use tempfile::tempdir;

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
    assert!(!script.contains("--exclude='*.md'"));
    assert!(!script.contains("--exclude='*.txt'"));
    assert!(script.contains("command -v python3"));
    assert!(script.contains("rg or grep is required"));
    assert!(script.contains("hygiene scan failed"));
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

    std::fs::write(plugin_root.join("README.md"), "AKIA1234567890ABCDEF\n")
        .expect("secret fixture");
    let secret_archive = root.path().join("secret.tar.gz");
    create_archive(root.path(), &secret_archive);
    assert!(!run_gate(&secret_archive, &plugin_root).status.success());
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

fn copy_tree(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            if entry.file_name() != "runtime" {
                copy_tree(&source_path, &target_path)?;
            }
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}
