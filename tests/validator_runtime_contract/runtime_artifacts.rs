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
        write_runtime_fixture(
            &plugin_root,
            runtime_name,
            &runtime_binary_fixture(runtime_name),
        )?;
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
        write_runtime_fixture(&plugin_root, runtime_name, b"#!/bin/sh\nexit 0\n")?;
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

#[test]
fn validator_cli_rejects_windows_runtime_without_a_valid_pe_signature()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    for runtime_name in packaged_runtime_names() {
        let bytes = if runtime_name == "codexy-mcp-lsp-windows-x86_64.exe" {
            let mut bytes = runtime_binary_fixture(runtime_name);
            bytes[0x80..0x84].copy_from_slice(b"PX\0\0");
            bytes
        } else {
            runtime_binary_fixture(runtime_name)
        };
        write_runtime_fixture(&plugin_root, runtime_name, &bytes)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("invalid binary format for windows-x86_64"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_package_without_native_windows_mcp_entrypoint()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    for runtime_name in packaged_runtime_names() {
        write_runtime_fixture(
            &plugin_root,
            runtime_name,
            &runtime_binary_fixture(runtime_name),
        )?;
    }
    std::fs::remove_file(plugin_root.join("mcp/codexy-mcp-lsp.exe"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("native Windows MCP entrypoint missing for lsp"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_non_x86_64_executable_pe_variants()
-> Result<(), Box<dyn std::error::Error>> {
    for scenario in ["x86", "pe32", "dll", "missing-optional-header", "truncated"] {
        let temp = tempfile::tempdir()?;
        let plugin_root = copy_plugin_to(temp.path())?;
        for runtime_name in packaged_runtime_names() {
            let mut bytes = runtime_binary_fixture(runtime_name);
            if runtime_name == "codexy-mcp-lsp-windows-x86_64.exe" {
                match scenario {
                    "x86" => bytes[0x84..0x86].copy_from_slice(&0x014c_u16.to_le_bytes()),
                    "pe32" => bytes[0x98..0x9a].copy_from_slice(&0x010b_u16.to_le_bytes()),
                    "dll" => bytes[0x96..0x98].copy_from_slice(&0x2022_u16.to_le_bytes()),
                    "missing-optional-header" => {
                        bytes[0x94..0x96].copy_from_slice(&0_u16.to_le_bytes());
                    }
                    "truncated" => bytes.truncate(0x90),
                    _ => unreachable!(),
                }
            }
            write_runtime_fixture(&plugin_root, runtime_name, &bytes)?;
        }

        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args([
                "--plugin-root",
                plugin_root.to_str().ok_or("plugin root path")?,
                "--check-runtime-artifacts",
            ])
            .output()?;
        assert!(!output.status.success(), "invalid PE passed: {scenario}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("invalid binary format for windows-x86_64"),
            "invalid PE failed for the wrong reason: {scenario}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
