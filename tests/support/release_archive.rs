use std::{
    fs::File,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const ARCHIVE_PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn assert_structured_literals(text: &str, rule_id: &str, required: &[&str]) {
    let missing: Vec<_> = required
        .iter()
        .filter(|literal| !text.contains(**literal))
        .collect();
    assert!(
        missing.is_empty(),
        "structured contract {rule_id} is missing required literals {missing:?}"
    );
}

#[allow(dead_code)]
pub(crate) fn assert_archive_scanner_contract(script: &str, checker: &str) {
    assert_structured_literals(
        script,
        "archive scanner behavior",
        &[
            "rg -a -n",
            "grep -a -Hn",
            "runtime/*.bin",
            "! -name '*.md'",
            "! -name '*.txt'",
            "command -v python3",
            "rg or grep is required",
            "hygiene scan failed",
            "duplicate archive entries",
            "unexpected runtime artifact",
            "unsafe archive path",
        ],
    );
    assert_structured_literals(
        checker,
        "MCP response checker behavior",
        &[
            "invalid JSON-RPC version for response id",
            "set(responses) != {1, 2}",
        ],
    );
}

#[allow(dead_code)]
pub(crate) fn assert_runtime_workflow_contract(workflow: &str) {
    assert_structured_literals(
        workflow,
        "runtime workflow coverage",
        &[
            "scripts/validate-plugin-config --plugin-root plugins/codexy --check\n          rsync -a",
            "Smoke test native MCP runtimes",
        ],
    );
}

pub(crate) fn copy_tree(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
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

pub(crate) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub(crate) fn create_archive(
    root: &std::path::Path,
    archive: &std::path::Path,
) -> std::io::Result<()> {
    create_archive_with_commands(root, archive, "tar", "gzip", ARCHIVE_PROCESS_TIMEOUT)
}

pub(crate) fn create_archive_with_commands(
    root: &std::path::Path,
    archive: &std::path::Path,
    tar_command: &str,
    gzip_command: &str,
    timeout: Duration,
) -> std::io::Result<()> {
    let archive_file = File::create(archive)?;
    let mut tar = Command::new(tar_command)
        .args(["-C"])
        .arg(root)
        .args(["-cf", "-", "plugins/codexy"])
        .stdout(Stdio::piped())
        .spawn()?;
    let tar_stdout = match tar.stdout.take() {
        Some(stdout) => stdout,
        None => {
            reap_archive_process(&mut tar);
            return Err(std::io::Error::other("tar stdout unavailable"));
        }
    };
    let mut gzip = match Command::new(gzip_command)
        .args(["-1", "-c"])
        .stdin(Stdio::from(tar_stdout))
        .stdout(archive_file)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            reap_archive_process(&mut tar);
            return Err(error);
        }
    };
    let gzip_status = match wait_for_archive_process(&mut gzip, "gzip", timeout) {
        Ok(status) => status,
        Err(error) => {
            reap_archive_process(&mut tar);
            return Err(error);
        }
    };
    let tar_status = wait_for_archive_process(&mut tar, "tar", timeout)?;
    if !gzip_status.success() {
        return Err(std::io::Error::other(format!("gzip failed: {gzip_status}")));
    }
    if !tar_status.success() {
        return Err(std::io::Error::other(format!("tar failed: {tar_status}")));
    }
    Ok(())
}

fn wait_for_archive_process(
    child: &mut Child,
    name: &str,
    timeout: Duration,
) -> std::io::Result<ExitStatus> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() >= timeout {
            reap_archive_process(child);
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("{name} timed out after {} seconds", timeout.as_secs_f32()),
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn reap_archive_process(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub(crate) fn complete_plugin_fixture(
    root: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = root.join("plugins/codexy");
    copy_tree(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let runtime = plugin_root.join("runtime");
    std::fs::create_dir_all(&runtime)?;
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
        .status()?;
    if !build.success() {
        return Err(std::io::Error::other(format!(
            "release runtime build failed: {build}"
        )));
    }
    let host_platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("linux", "x86_64") => "linux-x86_64",
        (os, architecture) => {
            return Err(std::io::Error::other(format!(
                "unsupported test host platform: {os}-{architecture}"
            )));
        }
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
                )?;
            } else {
                let header = if platform == "darwin-arm64" {
                    vec![0xcf, 0xfa, 0xed, 0xfe]
                } else {
                    vec![0x7f, b'E', b'L', b'F']
                };
                std::fs::write(&path, header.repeat(1024))?;
            }
            make_executable(&path)?;
        }
    }
    Ok(plugin_root)
}
