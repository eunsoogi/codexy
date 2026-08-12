use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Value, json};

#[test]
fn sync_version_cli_updates_only_the_supplied_isolated_root()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = super::shared_repository_archive()?;
    let diagnostic_root = super::archive_repository(archive, &temp, "diagnostic-root")?;
    let source_root = codexy_runtime::paths::repository_root();
    let source_root_before = version_surface_contents(source_root)?;
    select_next_public_identities(&diagnostic_root)?;
    let diagnostic_versions_before = version_surface_contents(&diagnostic_root)?;
    let bootstrap_before = bootstrap_surface_contents(&diagnostic_root)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", "1.3.1"])
        .env("CODEXY_REPO_ROOT", &diagnostic_root)
        .current_dir(&diagnostic_root)
        .output()?;
    assert!(
        output.status.success(),
        "isolated diagnostic failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(version_surface_contents(source_root)?, source_root_before);
    assert_eq!(
        bootstrap_surface_contents(&diagnostic_root)?,
        bootstrap_before
    );
    let diagnostic_versions_after = version_surface_contents(&diagnostic_root)?;
    assert_ne!(diagnostic_versions_after, diagnostic_versions_before);
    for (path, contents) in diagnostic_versions_after {
        let text = String::from_utf8_lossy(&contents);
        assert!(
            text.lines()
                .map(str::trim)
                .any(|line| matches!(line, "version = \"1.3.1\"" | "\"version\": \"1.3.1\",")),
            "supplied diagnostic root was not updated at {}",
            path.display()
        );
    }
    Ok(())
}

fn select_next_public_identities(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join(".agents/plugins/release-publish-contract.json");
    let mut contract: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    contract["bootstrap"]["selectedVersion"] = json!("1.3.1");
    contract["runtime"]["selectedTag"] = json!("v1.3.1");
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&contract)?))?;
    fs::write(
        root.join("packages/codexy-runtime/src/version/bootstrap.rs"),
        "pub(super) const VERSION: &str = \"1.3.1\";\npub(super) const CANDIDATE_VERSION: &str = \"1.3.0\";\n",
    )?;
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
            "packages/codexy-runtime/Cargo.lock",
            "packages/codexy-runtime/Cargo.toml",
            "plugins/codexy/.codex-plugin/plugin.json",
            "plugins/codexy-github/.codex-plugin/plugin.json",
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
