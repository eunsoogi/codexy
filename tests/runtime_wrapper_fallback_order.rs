#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::Command;

#[test]
fn mcp_wrappers_try_packaged_runtime_before_cargo_bootstrap()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let wrapper_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/mcp")
            .join(format!("codexy-mcp-{server}"));
        let wrapper = std::fs::read_to_string(&wrapper_path)?;

        assert_package_fallback_precedes_cargo_bootstrap(&wrapper, &wrapper_path)?;
    }

    Ok(())
}

fn assert_package_fallback_precedes_cargo_bootstrap(
    wrapper: &str,
    wrapper_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bundled_runtime_check = find_required(
        wrapper,
        "if [ -x \"$bundled_runtime\" ]; then",
        wrapper_path,
        "bundled runtime check",
    )?;
    let package_fallback = find_required(
        wrapper,
        "if [ \"$runtime_package_requested\" = 1 ]; then",
        wrapper_path,
        "package fallback",
    )?;
    let cargo_bootstrap = find_required(
        wrapper,
        "if [ \"$cargo_available\" = 1 ]; then\n  if [ \"$runtime_ref_is_pinned\" = 1 ]; then",
        wrapper_path,
        "Cargo bootstrap",
    )?;

    assert!(
        bundled_runtime_check < package_fallback,
        "{} should check bundled runtime before package fallback",
        wrapper_path.display()
    );
    assert!(
        package_fallback < cargo_bootstrap,
        "{} should try packaged runtime fallback before Cargo bootstrap",
        wrapper_path.display()
    );

    Ok(())
}

fn find_required(
    text: &str,
    needle: &str,
    wrapper_path: &Path,
    label: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    text.find(needle)
        .ok_or_else(|| format!("{} missing {label}: {needle}", wrapper_path.display()).into())
}

#[test]
fn mcp_wrapper_uses_package_runtime_without_invoking_cargo_when_package_exists()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("installed-plugin");
    let wrapper_dir = plugin_root.join("mcp");
    std::fs::create_dir_all(&wrapper_dir)?;
    let wrapper_path = wrapper_dir.join("codexy-mcp-lsp");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/mcp/codexy-mcp-lsp"),
        &wrapper_path,
    )?;
    make_executable(&wrapper_path)?;

    let package_archive = temp.path().join("codexy-marketplace-plugin.tar.gz");
    create_fake_runtime_package(temp.path(), &package_archive)?;
    let fake_cargo_dir = temp.path().join("fake-bin");
    let cargo_sentinel = temp.path().join("cargo-was-called");
    create_fake_cargo(&fake_cargo_dir)?;

    let output = Command::new(&wrapper_path)
        .arg("--stdio")
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .env("CODEXY_RUNTIME_PACKAGE_PATH", &package_archive)
        .env(
            "CODEXY_RUNTIME_CACHE_DIR",
            temp.path().join("runtime-cache"),
        )
        .env("CARGO_SENTINEL", &cargo_sentinel)
        .env(
            "PATH",
            format!(
                "{}:{}",
                fake_cargo_dir.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()?;

    assert_eq!(
        output.status.code(),
        Some(42),
        "wrapper should exec the packaged runtime\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PACKAGED_RUNTIME_USED"),
        "packaged runtime marker missing\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !cargo_sentinel.exists(),
        "Cargo should not be invoked when the packaged runtime is available"
    );

    Ok(())
}

fn create_fake_runtime_package(
    temp_root: &Path,
    archive_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let package_root = temp_root.join("package-root");
    let runtime_dir = package_root.join("runtime");
    std::fs::create_dir_all(&runtime_dir)?;
    let runtime_path = runtime_dir.join("codexy-mcp-lsp-darwin-arm64.bin");
    std::fs::write(
        &runtime_path,
        "#!/bin/sh\necho PACKAGED_RUNTIME_USED \"$@\" >&2\nexit 42\n",
    )?;
    make_executable(&runtime_path)?;

    let status = Command::new("tar")
        .args(["-czf"])
        .arg(archive_path)
        .arg("-C")
        .arg(&package_root)
        .arg(".")
        .status()?;
    assert!(status.success(), "tar should create runtime package");

    Ok(())
}

fn create_fake_cargo(fake_bin_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(fake_bin_dir)?;
    let cargo_path = fake_bin_dir.join("cargo");
    std::fs::write(
        &cargo_path,
        "#!/bin/sh\necho cargo > \"$CARGO_SENTINEL\"\nexit 86\n",
    )?;
    make_executable(&cargo_path)?;
    Ok(())
}

fn make_executable(path: &Path) -> std::io::Result<()> {
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
