use super::*;

pub(super) fn installed_plugin_copy() -> Result<InstalledPlugin, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let installed_plugin = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &installed_plugin,
    )?;
    install_runtime_fixture(
        &installed_plugin,
        "codexy-mcp-lsp",
        env!("CARGO_BIN_EXE_codexy-mcp-lsp"),
    )?;
    install_runtime_fixture(
        &installed_plugin,
        "codexy-mcp-codegraph",
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
    )?;
    Ok(InstalledPlugin {
        _temp: temp,
        path: installed_plugin,
    })
}

pub(super) fn installed_plugin_under_rust_host()
-> Result<InstalledPlugin, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let host = temp.path().join("host");
    let installed_plugin = host.join("plugins/codexy");
    std::fs::create_dir_all(host.join("src"))?;
    std::fs::write(
        host.join("Cargo.toml"),
        "[package]\nname = \"host-project\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &installed_plugin,
    )?;
    Ok(InstalledPlugin {
        _temp: temp,
        path: installed_plugin,
    })
}

pub(super) fn temp_runtime_dir(
    runtime_name: &str,
    source_binary: &str,
) -> Result<TempRuntimeDir, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let runtime_dir = temp.path().join("runtimes");
    std::fs::create_dir_all(&runtime_dir)?;
    let runtime_path = runtime_dir.join(runtime_name);
    std::fs::copy(source_binary, &runtime_path)?;
    let mut permissions = std::fs::metadata(&runtime_path)?.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        permissions.set_mode(0o755);
    }
    std::fs::set_permissions(&runtime_path, permissions)?;
    Ok(TempRuntimeDir {
        _temp: temp,
        path: runtime_dir,
    })
}

pub(super) fn install_runtime_fixture(
    installed_plugin: &Path,
    runtime: &str,
    source_binary: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime_dir = installed_plugin.join("runtime");
    std::fs::create_dir_all(&runtime_dir)?;
    for platform in ["darwin-arm64", "linux-x86_64"] {
        let runtime_path = runtime_dir.join(format!("{runtime}-{platform}.bin"));
        std::fs::copy(source_binary, &runtime_path)?;
        let mut permissions = std::fs::metadata(&runtime_path)?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            permissions.set_mode(0o755);
        }
        std::fs::set_permissions(&runtime_path, permissions)?;
    }
    Ok(())
}

pub(super) fn copy_dir(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}
