use std::process::Command;

#[test]
fn validator_cli_checks_all_contract_surfaces() -> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "--check",
        "--check-mcp",
        "--check-lsp",
        "--check-roles",
        "--print-covered-extensions",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .arg(mode)
            .output()?;
        assert!(
            output.status.success(),
            "validator {mode} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn sync_version_cli_checks_manifest_marketplace_parity() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .arg("--check")
        .output()?;
    assert!(
        output.status.success(),
        "sync-version --check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("plugin version sync ok"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}
