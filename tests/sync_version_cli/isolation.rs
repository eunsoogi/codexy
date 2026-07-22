use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn sync_version_cli_updates_only_the_supplied_isolated_root()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let build_root = super::archive_repository(&temp, "build-root")?;
    let diagnostic_root = super::archive_repository(&temp, "diagnostic-root")?;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/paths.rs"),
        build_root.join("src/paths.rs"),
    )?;
    let build_target = build_root.join("target");

    let build_status = Command::new("cargo")
        .args([
            "build",
            "--locked",
            "--quiet",
            "--no-default-features",
            "--bin",
            "codexy-sync-version",
        ])
        .env("CARGO_TARGET_DIR", &build_target)
        .current_dir(&build_root)
        .status()?;
    assert!(build_status.success(), "isolated helper build failed");

    let build_root_before = version_surface_contents(&build_root)?;
    super::admission::activate(&diagnostic_root)?;
    let bootstrap_before = bootstrap_surface_contents(&diagnostic_root)?;
    let output = Command::new(build_target.join("debug/codexy-sync-version"))
        .args(["--version", "1.3.0"])
        .env("CODEXY_REPO_ROOT", &diagnostic_root)
        .current_dir(&diagnostic_root)
        .output()?;
    assert!(
        output.status.success(),
        "isolated diagnostic failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(version_surface_contents(&build_root)?, build_root_before);
    assert_eq!(
        bootstrap_surface_contents(&diagnostic_root)?,
        bootstrap_before
    );
    for (path, contents) in version_surface_contents(&diagnostic_root)? {
        let text = String::from_utf8_lossy(&contents);
        assert!(
            text.lines()
                .map(str::trim)
                .any(|line| matches!(line, "version = \"1.3.0\"" | "\"version\": \"1.3.0\",")),
            "supplied diagnostic root was not updated at {}",
            path.display()
        );
    }
    Ok(())
}

pub(super) fn version_surface_contents(
    root: &Path,
) -> Result<Vec<(PathBuf, Vec<u8>)>, Box<dyn std::error::Error>> {
    contents(
        root,
        [
            ".agents/plugins/marketplace.json",
            ".agents/plugins/release-publish-contract.json",
            "Cargo.lock",
            "Cargo.toml",
            "plugins/codexy/.codex-plugin/plugin.json",
        ],
    )
}

fn bootstrap_surface_contents(
    root: &Path,
) -> Result<Vec<(PathBuf, Vec<u8>)>, Box<dyn std::error::Error>> {
    contents(
        root,
        [
            "packages/getcodexy/pyproject.toml",
            "plugins/codexy/mcp/codexy-mcp-lsp",
            "plugins/codexy/mcp/codexy-mcp-codegraph",
        ],
    )
}

fn contents<const N: usize>(
    root: &Path,
    paths: [&str; N],
) -> Result<Vec<(PathBuf, Vec<u8>)>, Box<dyn std::error::Error>> {
    paths
        .into_iter()
        .map(|relative| {
            let path = root.join(relative);
            Ok((path.clone(), fs::read(path)?))
        })
        .collect()
}
