use super::*;

const LEGACY_PUBLIC_PLATFORMS: &[&str] = &["darwin-arm64", "linux-x86_64"];

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
    let candidate_runtime_dir = install_windows_candidate_runtime_fixture(temp.path())?;
    Ok(InstalledPlugin {
        _temp: temp,
        path: installed_plugin,
        candidate_runtime_dir,
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
        candidate_runtime_dir: None,
    })
}

fn install_windows_candidate_runtime_fixture(
    root: &Path,
) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let runtime_dir = root.join("candidate-runtime");
        std::fs::create_dir_all(&runtime_dir)?;
        for (runtime, source_binary) in [
            ("codexy-mcp-lsp", env!("CARGO_BIN_EXE_codexy-mcp-lsp")),
            ("codexy-mcp-codegraph", env!("CARGO_BIN_EXE_codexy-mcp-codegraph")),
        ] {
            std::fs::copy(
                source_binary,
                runtime_dir.join(windows_candidate_runtime_name(runtime)),
            )?;
        }
        return Ok(Some(runtime_dir));
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        Ok(None)
    }
}

fn windows_candidate_runtime_name(runtime: &str) -> String {
    format!("{runtime}-windows-x86_64.exe")
}

#[test]
fn windows_candidate_fixture_name_keeps_legacy_public_runtime_names_unchanged() {
    assert_eq!(
        windows_candidate_runtime_name("codexy-mcp-lsp"),
        "codexy-mcp-lsp-windows-x86_64.exe"
    );
    assert_eq!(LEGACY_PUBLIC_PLATFORMS, ["darwin-arm64", "linux-x86_64"]);
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
    for platform in LEGACY_PUBLIC_PLATFORMS {
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
