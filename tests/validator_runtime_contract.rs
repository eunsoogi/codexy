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
