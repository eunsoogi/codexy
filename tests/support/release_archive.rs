use std::{
    fs::File,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

#[path = "pe_fixture.rs"]
mod pe_fixture;
#[path = "release_archive_contract.rs"]
mod release_archive_contract;
#[allow(unused_imports)]
pub(crate) use release_archive_contract::{
    assert_archive_scanner_contract, assert_runtime_workflow_contract, assert_structured_literals,
};

const ARCHIVE_PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

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

pub(crate) fn archive_gate_with_test_validator(
    root: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    let scripts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
    let gate = root.join("inspect-release-archive");
    let max_runtime_bytes = max_test_runtime_bytes()?;
    let source = std::fs::read_to_string(scripts.join("inspect-release-archive"))?;
    let source = source
        .strip_prefix("#!/bin/sh\n")
        .expect("archive gate must use a POSIX shell shebang");
    std::fs::write(
        &gate,
        format!(
            "#!/bin/sh\n# Cargo test binaries include debug metadata; production defaults remain in the source gate.\nMAX_ARCHIVE_FILE_BYTES={}\nMAX_ARCHIVE_TOTAL_BYTES={}\nexport MAX_ARCHIVE_FILE_BYTES MAX_ARCHIVE_TOTAL_BYTES\n{source}",
            max_runtime_bytes,
            max_runtime_bytes.saturating_mul(3),
        ),
    )?;
    std::fs::copy(
        scripts.join("inspect-mcp-response"),
        root.join("inspect-mcp-response"),
    )?;
    std::fs::copy(
        env!("CARGO_BIN_EXE_codexy-validate"),
        root.join("validate-plugin-config"),
    )?;
    make_executable(&gate)?;
    make_executable(&root.join("inspect-mcp-response"))?;
    make_executable(&root.join("validate-plugin-config"))?;
    Ok(gate)
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
        .env("COPYFILE_DISABLE", "1")
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
        (
            "lsp",
            std::path::Path::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp")),
        ),
        (
            "codegraph",
            std::path::Path::new(env!("CARGO_BIN_EXE_codexy-mcp-codegraph")),
        ),
    ] {
        for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
            let extension = if platform == "windows-x86_64" {
                "exe"
            } else {
                "bin"
            };
            let path = runtime.join(format!("codexy-mcp-{server}-{platform}.{extension}"));
            let bytes = if platform == host_platform {
                std::fs::read(binary)?
            } else {
                match platform {
                    "darwin-arm64" => vec![0xcf, 0xfa, 0xed, 0xfe],
                    "linux-x86_64" => vec![0x7f, b'E', b'L', b'F'],
                    "windows-x86_64" => pe_fixture::x86_64_executable(),
                    _ => unreachable!(),
                }
            };
            std::fs::write(&path, bytes)?;
            make_executable(&path)?;
            if platform == "windows-x86_64" {
                std::fs::copy(
                    &path,
                    plugin_root.join(format!("mcp/codexy-mcp-{server}.exe")),
                )?;
            }
        }
    }
    Ok(plugin_root)
}

fn max_test_runtime_bytes() -> std::io::Result<u64> {
    [
        env!("CARGO_BIN_EXE_codexy-mcp-lsp"),
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
    ]
    .into_iter()
    .map(|binary| std::fs::metadata(binary).map(|metadata| metadata.len().saturating_add(1)))
    .collect::<std::io::Result<Vec<_>>>()
    .map(|sizes| sizes.into_iter().max().unwrap_or(1))
}
