#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

#[path = "validator_runtime_contract/platform_matrix.rs"]
mod platform_matrix;
#[path = "validator_runtime_contract/runtime_artifacts.rs"]
mod runtime_artifacts;

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
        return bytes;
    } else if runtime_name.contains("darwin-arm64") {
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

fn write_runtime_fixture(
    plugin_root: &std::path::Path,
    runtime_name: &str,
    bytes: &[u8],
) -> std::io::Result<()> {
    let runtime_path = plugin_root.join("runtime").join(runtime_name);
    std::fs::create_dir_all(runtime_path.parent().expect("runtime parent"))?;
    std::fs::write(&runtime_path, bytes)?;
    make_executable(&runtime_path)?;
    if runtime_name.contains("windows-x86_64") {
        let server = if runtime_name.contains("-lsp-") {
            "lsp"
        } else {
            "codegraph"
        };
        std::fs::write(
            plugin_root.join(format!("mcp/codexy-mcp-{server}.exe")),
            bytes,
        )?;
    }
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
