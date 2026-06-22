use std::process::Command;

#[test]
fn validator_cli_accepts_available_rust_analyzer() -> Result<(), Box<dyn std::error::Error>> {
    let path_dir = tempfile::tempdir()?;
    let rust_analyzer = path_dir.path().join("rust-analyzer");
    std::fs::write(&rust_analyzer, "#!/bin/sh\nexit 0\n")?;
    make_executable(&rust_analyzer)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--check-rust-lsp-readiness")
        .env("PATH", path_dir.path())
        .output()?;

    assert!(
        output.status.success(),
        "validator should pass when rust-analyzer is executable on PATH\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_reports_missing_rust_analyzer_install_action()
-> Result<(), Box<dyn std::error::Error>> {
    let path_dir = tempfile::tempdir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--check-rust-lsp-readiness")
        .env("PATH", path_dir.path())
        .output()?;

    assert!(
        !output.status.success(),
        "validator should fail when rust-analyzer is absent from PATH"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr
            .contains("Rust LSP command unavailable: executable not found on PATH: rust-analyzer"),
        "stderr should identify the missing rust-analyzer executable, got:\n{stderr}"
    );
    assert!(
        stderr.contains("install rust-analyzer, for example with `rustup component add rust-analyzer`, or put rust-analyzer on PATH before PR readiness"),
        "stderr should include the concrete install/config action, got:\n{stderr}"
    );
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt as _;

    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
