use std::{fs, path::Path};

use crate::support::FixtureCommand;
use serde_json::{Value, json};

use super::{archive_repository, shared_repository_archive};

#[test]
fn sync_version_check_rejects_duplicate_component_manifest_keys_without_mutating()
-> Result<(), Box<dyn std::error::Error>> {
    for (label, needle) in [
        ("top-level", "\"schema\": \"getcodexy.component-manifest.v1\","),
        ("nested", "\"pluginId\": \"codexy@codexy\","),
    ] {
        let temp = tempfile::tempdir()?;
        let repo = archive_repository(shared_repository_archive()?, &temp, label)?;
        let path = repo.join("packages/getcodexy/src/codexy_runtime_tools/component-manifest.json");
        let before = fs::read_to_string(&path)?;
        fs::write(&path, before.replacen(needle, &format!("{needle} {needle}"), 1))?;

        let output = FixtureCommand::new(repo.join("scripts/sync-plugin-version.sh"))
            .arg("--check")
            .current_dir(&repo)
            .output()?;
        assert!(!output.status.success(), "{label} duplicate unexpectedly passed");
        assert_ne!(fs::read_to_string(&path)?, before, "--check rewrote {label} fixture");
    }
    Ok(())
}

pub(super) fn select_version_advance(
    root: &Path,
    target: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let contract = root.join(".agents/plugins/release-publish-contract.json");
    let mut data: Value = serde_json::from_str(&fs::read_to_string(&contract)?)?;
    data["bootstrap"]["selectedVersion"] = json!(target);
    data["runtime"]["selectedTag"] = json!(format!("v{target}"));
    fs::write(contract, format!("{}\n", serde_json::to_string_pretty(&data)?))?;
    let candidate = super::isolation::bootstrap_candidate_version(root)?;
    fs::write(
        root.join("packages/codexy-runtime/src/version/bootstrap.rs"),
        format!(
            "pub(super) const VERSION: &str = \"{target}\";\npub(super) const CANDIDATE_VERSION: &str = \"{candidate}\";\n"
        ),
    )?;
    Ok(())
}
