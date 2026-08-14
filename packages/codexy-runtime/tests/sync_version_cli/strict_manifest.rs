use std::{fs, path::Path, process::Command};

use crate::support::FixtureCommand;
use serde_json::{Value, json};

use super::{archive_repository, shared_repository_archive};

#[test]
fn sync_version_check_rejects_duplicate_component_manifest_keys_without_mutating()
-> Result<(), Box<dyn std::error::Error>> {
    for (label, needle) in [
        (
            "top-level",
            "\"schema\": \"getcodexy.component-manifest.v1\",",
        ),
        ("nested", "\"pluginId\": \"codexy@codexy\","),
    ] {
        let temp = tempfile::tempdir()?;
        let repo = archive_repository(shared_repository_archive()?, &temp, label)?;
        let path = repo.join("packages/getcodexy/src/codexy_runtime_tools/component-manifest.json");
        let before = fs::read_to_string(&path)?;
        fs::write(
            &path,
            before.replacen(needle, &format!("{needle} {needle}"), 1),
        )?;

        let output = FixtureCommand::new(repo.join("scripts/sync-plugin-version"))
            .arg("--check")
            .current_dir(&repo)
            .output()?;
        assert!(
            !output.status.success(),
            "{label} duplicate unexpectedly passed"
        );
        assert_ne!(
            fs::read_to_string(&path)?,
            before,
            "--check rewrote {label} fixture"
        );
    }
    Ok(())
}

#[test]
fn sync_version_check_rejects_a_stale_root_package_lock_without_mutating()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(shared_repository_archive()?, &temp, "package-lock")?;
    let path = repo.join("package-lock.json");
    let before = fs::read_to_string(&path)?;
    let mut lock: Value = serde_json::from_str(&before)?;
    lock["packages"][""]["version"] = json!("9.9.9");
    fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&lock)?))?;
    let stale = fs::read_to_string(&path)?;

    let output = FixtureCommand::new(repo.join("scripts/sync-plugin-version"))
        .arg("--check")
        .current_dir(&repo)
        .output()?;
    assert!(
        !output.status.success(),
        "stale package lock unexpectedly passed"
    );
    assert_eq!(
        fs::read_to_string(&path)?,
        stale,
        "--check rewrote package lock"
    );
    Ok(())
}

#[test]
fn version_admission_rejects_out_of_range_semver_before_mutation()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(shared_repository_archive()?, &temp, "overflow")?;
    let target = "2147483648.0.0";
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--admit-version", target])
        .env("CODEXY_REPO_ROOT", &repo)
        .current_dir(&repo)
        .output()?;
    assert!(
        !output.status.success(),
        "overflow version unexpectedly admitted"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("version must be semver-like"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn version_admission_accepts_the_semver_component_upper_bound()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(shared_repository_archive()?, &temp, "upper-bound")?;
    let target = "2147483647.0.0";
    select_version_advance(&repo, target)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--admit-version", target])
        .env("CODEXY_REPO_ROOT", &repo)
        .current_dir(&repo)
        .output()?;
    assert!(
        output.status.success(),
        "upper-bound version unexpectedly rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
    fs::write(
        contract,
        format!("{}\n", serde_json::to_string_pretty(&data)?),
    )?;
    fs::write(
        root.join("packages/codexy-runtime/src/version/bootstrap.rs"),
        format!(
            "pub(super) const VERSION: &str = \"{target}\";\npub(super) const CANDIDATE_VERSION: &str = \"1.3.0\";\n"
        ),
    )?;
    Ok(())
}
