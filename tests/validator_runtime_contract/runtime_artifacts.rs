use super::*;

#[test]
fn validator_cli_rejects_packaged_plugin_without_generated_runtime_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "packaged artifact validation should reject missing generated runtime binaries"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("codexy-mcp-lsp-darwin-arm64.bin bundled MCP runtime missing"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_packaged_plugin_with_generated_runtime_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    for runtime_name in packaged_runtime_names() {
        let runtime_path = plugin_root.join("runtime").join(runtime_name);
        std::fs::create_dir_all(runtime_path.parent().ok_or("runtime parent")?)?;
        std::fs::write(&runtime_path, runtime_binary_fixture(runtime_name))?;
        make_executable(&runtime_path)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "packaged artifact validation should accept generated runtime binaries\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_packaged_plugin_with_script_placeholders()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    for runtime_name in packaged_runtime_names() {
        let runtime_path = plugin_root.join("runtime").join(runtime_name);
        std::fs::create_dir_all(runtime_path.parent().ok_or("runtime parent")?)?;
        std::fs::write(&runtime_path, "#!/bin/sh\nexit 0\n")?;
        make_executable(&runtime_path)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "packaged artifact validation should reject script placeholders"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("bundled MCP runtime has invalid binary format"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
