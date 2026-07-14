use std::process::Command;

pub fn copy_tree(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
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

pub fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub fn complete_plugin_fixture(root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
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
