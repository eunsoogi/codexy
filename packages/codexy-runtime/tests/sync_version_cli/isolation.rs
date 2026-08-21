use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Value, json};

pub(super) fn fixture_version(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest: Value = serde_json::from_str(&fs::read_to_string(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    manifest["version"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "manifest version".into())
}

pub(super) fn bootstrap_candidate_version(
    root: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(root.join("packages/codexy-runtime/src/version/bootstrap.rs"))?;
    let prefix = "pub(super) const CANDIDATE_VERSION: &str = \"";
    text.lines()
        .find_map(|line| line.strip_prefix(prefix)?.strip_suffix("\";"))
        .map(ToOwned::to_owned)
        .ok_or_else(|| "candidate bootstrap version".into())
}

pub(super) fn prior_runtime_version(
    root: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let release: Value = serde_json::from_str(&fs::read_to_string(
        root.join("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    release["artifact"]["tag"]
        .as_str()
        .and_then(|tag| tag.strip_prefix('v'))
        .map(ToOwned::to_owned)
        .ok_or_else(|| "prior runtime version".into())
}

pub(super) fn next_patch_version(
    version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!("version is not MAJOR.MINOR.PATCH: {version}").into());
    }
    let major: u64 = parts[0].parse()?;
    let minor: u64 = parts[1].parse()?;
    let patch: u64 = parts[2].parse()?;
    let next = patch.checked_add(1).ok_or("patch version overflow")?;
    Ok(format!("{major}.{minor}.{next}"))
}

#[test]
fn sync_version_cli_updates_only_the_supplied_isolated_root()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = super::shared_repository_archive()?;
    let diagnostic_root = super::archive_repository(archive, &temp, "diagnostic-root")?;
    let source_root = codexy_runtime::paths::repository_root();
    let selected_version = fixture_version(&diagnostic_root)?;
    let target = next_patch_version(&selected_version)?;
    let source_root_before = version_surface_contents(source_root)?;
    select_next_public_identities(&diagnostic_root, &target, &selected_version)?;
    let diagnostic_versions_before = version_surface_contents(&diagnostic_root)?;
    let bootstrap_before = bootstrap_surface_contents(&diagnostic_root)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", target.as_str()])
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
    let contract: Value = serde_json::from_slice(&fs::read(
        diagnostic_root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    assert_eq!(contract["currentMarketplace"]["ref"], format!("v{target}"));
    assert_eq!(
        contract["currentMarketplace"]["installCommand"],
        format!("codex plugin marketplace add eunsoogi/codexy --ref v{target}")
    );
    for (path, contents) in diagnostic_versions_after {
        let text = String::from_utf8_lossy(&contents);
        assert!(
            text.lines()
            .map(str::trim)
                .any(|line| {
                    line == format!("version = \"{target}\"")
                        || line == format!("\"version\": \"{target}\",")
                }),
            "supplied diagnostic root was not updated at {}",
            path.display()
        );
    }
    Ok(())
}

fn select_next_public_identities(
    root: &Path,
    target: &str,
    candidate: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join(".agents/plugins/release-publish-contract.json");
    let mut contract: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
    contract["bootstrap"]["selectedVersion"] = json!(target);
    contract["runtime"]["selectedTag"] = json!(format!("v{target}"));
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&contract)?))?;
    fs::write(
        root.join("packages/codexy-runtime/src/version/bootstrap.rs"),
        format!(
            "pub(super) const VERSION: &str = \"{target}\";\npub(super) const CANDIDATE_VERSION: &str = \"{candidate}\";\n"
        ),
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
            "packages/getcodexy/pyproject.toml",
            "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
            "packages/getcodexy/uv.lock",
            "plugins/codexy/.codex-plugin/plugin.json",
            "plugins/codexy-devtools/.codex-plugin/plugin.json",
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
            "plugins/codexy-devtools/mcp/codexy-mcp-devtools",
            "plugins/codexy-devtools/mcp/codexy-mcp-lsp",
            "plugins/codexy-devtools/mcp/codexy-mcp-codegraph",
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
