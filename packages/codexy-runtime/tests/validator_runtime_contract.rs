#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

use crate::support;

#[path = "validator_runtime_contract/platform_matrix.rs"]
mod platform_matrix;
#[path = "validator_runtime_contract/runtime_artifacts.rs"]
mod runtime_artifacts;
#[path = "validator_runtime_contract/release_contract.rs"]
mod release_contract;

fn copy_plugin_to(temp_root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = temp_root.join("codexy-devtools");
    support::copy_devtools_fixture_into_with_mutable_files(
        &plugin_root,
        &[
            std::path::Path::new(".codex-plugin/plugin.json"),
            std::path::Path::new("runtime-release.json"),
            std::path::Path::new("mcp/codexy-mcp-lsp"),
            std::path::Path::new("mcp/codexy-mcp-codegraph"),
            std::path::Path::new("mcp/codexy-mcp-devtools"),
        ],
    )?;
    Ok(plugin_root)
}

const fn packaged_runtime_names() -> [&'static str; 6] {
    [
        "codexy-mcp-lsp-darwin-arm64.bin",
        "codexy-mcp-codegraph-darwin-arm64.bin",
        "codexy-mcp-lsp-linux-x86_64.bin",
        "codexy-mcp-codegraph-linux-x86_64.bin",
        "codexy-mcp-lsp-windows-x86_64.exe",
        "codexy-mcp-codegraph-windows-x86_64.exe",
    ]
}

fn runtime_binary_fixture(runtime_name: &str) -> Vec<u8> {
    let mut bytes = if runtime_name.contains("windows-x86_64") {
        let mut bytes = vec![0; 4096];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&(0x80_u32).to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&0x8664_u16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&1_u16.to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&0xf0_u16.to_le_bytes());
        bytes[0x96..0x98].copy_from_slice(&0x0022_u16.to_le_bytes());
        bytes[0x98..0x9a].copy_from_slice(&0x20b_u16.to_le_bytes());
        bytes
    } else if runtime_name.contains("darwin-arm64") {
        vec![0xcf, 0xfa, 0xed, 0xfe]
    } else {
        vec![0x7f, b'E', b'L', b'F']
    };
    bytes.resize(4096, 0);
    if runtime_name.contains("codegraph") {
        bytes[0x100] = 1;
    }
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

fn write_runtime_fixture(
    plugin_root: &std::path::Path,
    runtime_name: &str,
    bytes: &[u8],
) -> std::io::Result<()> {
    let runtime_path = plugin_root.join("runtime").join(runtime_name);
    std::fs::create_dir_all(runtime_path.parent().expect("runtime parent"))?;
    std::fs::write(&runtime_path, bytes)?;
    make_executable(&runtime_path)?;
    if runtime_name == "codexy-mcp-lsp-windows-x86_64.exe" {
        let mut dispatcher = bytes.to_vec();
        dispatcher[0x100] = 2;
        std::fs::write(plugin_root.join("mcp/codexy-mcp-devtools.exe"), dispatcher)?;
    }
    Ok(())
}
