#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

#[test]
fn validator_cli_rejects_supported_platform_without_bundled_mcp_runtimes()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] =
        serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject advertised platforms without bundled runtimes"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("bundled platforms for lsp must match"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_supported_platform_without_build_matrix_coverage()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] =
        serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    for server in ["lsp", "codegraph"] {
        let wrapper_path = plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        let wrapper = std::fs::read_to_string(&wrapper_path)?.replace(
            "bundled_platforms=\"darwin-arm64 linux-x86_64\"",
            "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
        );
        std::fs::write(&wrapper_path, wrapper)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject advertised platforms without release matrix coverage"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("runtime build matrix must cover supported platform windows-x86_64"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_platform_narrowing_without_required_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] = serde_json::json!(["darwin-arm64"]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    for server in ["lsp", "codegraph"] {
        let wrapper_path = plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        let wrapper = std::fs::read_to_string(&wrapper_path)?.replace(
            "bundled_platforms=\"darwin-arm64 linux-x86_64\"",
            "bundled_platforms=\"darwin-arm64\"",
        );
        std::fs::write(&wrapper_path, wrapper)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject narrowing below the baseline supported platforms"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("supportedPlatforms must include linux-x86_64"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

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

fn copy_plugin_to(temp_root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = temp_root.join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

const fn packaged_runtime_names() -> [&'static str; 4] {
    [
        "codexy-mcp-lsp-darwin-arm64.bin",
        "codexy-mcp-codegraph-darwin-arm64.bin",
        "codexy-mcp-lsp-linux-x86_64.bin",
        "codexy-mcp-codegraph-linux-x86_64.bin",
    ]
}

fn runtime_binary_fixture(runtime_name: &str) -> Vec<u8> {
    let mut bytes = if runtime_name.contains("darwin-arm64") {
        vec![0xcf, 0xfa, 0xed, 0xfe]
    } else {
        vec![0x7f, b'E', b'L', b'F']
    };
    bytes.resize(4096, 0);
    bytes
}

fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn copy_dir(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            if entry.file_name() == "target" {
                continue;
            }
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}
