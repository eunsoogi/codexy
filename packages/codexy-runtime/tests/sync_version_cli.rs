use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

use crate::support::FixtureCommand;

#[path = "sync_version_cli/isolation.rs"]
mod isolation;
#[path = "sync_version_cli/mutation_preflight.rs"]
mod mutation_preflight;
#[path = "sync_version_cli/admission.rs"]
mod admission;
#[path = "sync_version_cli/archive.rs"]
mod archive;
#[path = "sync_version_cli/candidate_negatives.rs"]
mod candidate_negatives;
#[path = "sync_version_cli/fixture_files.rs"]
mod fixture_files;
#[path = "sync_version_cli/strict_manifest.rs"]
mod strict_manifest;
#[path = "sync_version_cli/uv_lock.rs"]
mod uv_lock;

pub(super) use archive::{archive_repository, shared_repository_archive};

#[test]
fn sync_version_cli_checks_manifest_marketplace_parity() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let (root, version) = selected_fixture(&temp, "manifest-parity")?;
    let output = run_sync(&root, &["--check"])?;
    assert!(
        output.status.success(),
        "sync-version --check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!(
                "plugin version sync ok: codexy={version}, codexy-github={version}"
            )),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    for relative in ["README.md", "README.ko.md"] {
        assert!(fs::read_to_string(root.join(relative))?.contains(&format!("--ref v{version}")));
    }
    Ok(())
}

#[test]
fn sync_version_script_rejects_malformed_readme_pins_without_mutation()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    for (case, candidate_mode) in [
        ("stale-prior", false),
        ("stale-candidate", true),
        ("missing", false),
        ("missing-v", false),
        ("invalid-semver", false),
        ("empty", false),
        ("duplicate", false),
    ] {
        for (index, relative) in ["README.md", "README.ko.md"].into_iter().enumerate() {
            let (root, selected) = selected_fixture(&temp, &format!("readme-{case}-{index}"))?;
            let prior = previous_patch_version(&selected)?;
            let candidate = isolation::next_patch_version(&selected)?;
            if candidate_mode {
                let prepared = run_sync_script(&root, &["--prepare-candidate", &candidate])?;
                assert!(
                    prepared.status.success(),
                    "candidate preparation failed: {}",
                    String::from_utf8_lossy(&prepared.stderr)
                );
            }
            let expected = if candidate_mode { &candidate } else { &selected };
            let direct_pin = format!("{README_COMMAND} --ref v{expected}");
            let (replacement, diagnostic) = match case {
                "stale-prior" => (format!("{README_COMMAND} --ref v{prior}"), "version mismatch"),
                "stale-candidate" => (
                    format!("{README_COMMAND} --ref v{selected}"),
                    "version mismatch",
                ),
                "missing" => (README_COMMAND.to_owned(), "exactly one direct marketplace pin"),
                "missing-v" => (format!("{README_COMMAND} --ref {expected}"), "must start with v"),
                "invalid-semver" => (
                    format!("{README_COMMAND} --ref vnot-a-version"),
                    "semver-like MAJOR.MINOR.PATCH",
                ),
                "empty" => (format!("{README_COMMAND} --ref "), "marketplace pin is empty"),
                "duplicate" => (
                    format!("{direct_pin}\n{direct_pin}"),
                    "exactly one direct marketplace pin",
                ),
                other => return Err(format!("unknown README case: {other}").into()),
            };
            let path = root.join(relative);
            let original = fs::read(&path)?;
            let mutated = String::from_utf8(original.clone())?.replace(&direct_pin, &replacement);
            assert_ne!(mutated.as_bytes(), original.as_slice(), "fixture pin was not found");
            fs::write(&path, mutated.as_bytes())?;
            let check = if candidate_mode { "--check-candidate" } else { "--check" };
            let output = run_sync_script(&root, &[check])?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success(), "{case} {relative} unexpectedly passed");
            assert!(stderr.contains(relative), "{case} diagnostic omitted {relative}: {stderr}");
            assert!(stderr.contains(diagnostic), "{case} diagnostic changed: {stderr}");
            assert_eq!(fs::read(&path)?, mutated.as_bytes(), "README was mutated");
        }
    }
    Ok(())
}

#[test]
fn sync_version_cli_checks_release_tag_parity() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let (root, version) = selected_fixture(&temp, "tag-parity")?;
    let matching_tag = format!("v{version}");
    let matching = run_sync(&root, &["--check", "--tag", &matching_tag])?;
    assert!(
        matching.status.success(),
        "matching release tag failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&matching.stdout),
        String::from_utf8_lossy(&matching.stderr)
    );

    let mismatched = run_sync(&root, &["--check", "--tag", version.as_str()])?;
    assert!(
        !mismatched.status.success(),
        "tag without v prefix unexpectedly passed"
    );

    let stale_tag = format!("v{}", isolation::next_patch_version(&version)?);
    let stale = run_sync(&root, &["--check", "--tag", &stale_tag])?;
    assert!(
        !stale.status.success(),
        "mismatched release tag unexpectedly passed"
    );
    Ok(())
}

fn run_sync(root: &std::path::Path, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(args)
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()?)
}

const README_COMMAND: &str = "codex plugin marketplace add eunsoogi/codexy";

fn run_sync_script(root: &std::path::Path, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    fs::copy(
        codexy_runtime::paths::repository_root().join("scripts/sync-plugin-version.sh"),
        root.join("scripts/sync-plugin-version.sh"),
    )?;
    Ok(FixtureCommand::new(root.join("scripts/sync-plugin-version.sh"))
        .args(args)
        .current_dir(root)
        .output()?)
}

fn previous_patch_version(version: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (prefix, patch) = version.rsplit_once('.').ok_or("version")?;
    Ok(format!(
        "{prefix}.{}",
        patch.parse::<u64>()?.checked_sub(1).ok_or("version underflow")?
    ))
}

fn fixture_repo(
    temp: &tempfile::TempDir,
    name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    archive_repository(shared_repository_archive()?, temp, name)
}

fn selected_fixture(
    temp: &tempfile::TempDir,
    name: &str,
) -> Result<(PathBuf, String), Box<dyn std::error::Error>> {
    let root = fixture_repo(temp, name)?;
    fs::copy(
        codexy_runtime::paths::repository_root().join("packages/codexy-runtime/src/version/readme.rs"),
        root.join("packages/codexy-runtime/src/version/readme.rs"),
    )?;
    let manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let version = manifest["version"]
        .as_str()
        .ok_or("manifest version")?
        .to_owned();
    let normalized = run_sync(&root, &["--version", &version])?;
    if !normalized.status.success() {
        return Err(format!(
            "selected fixture normalization failed: {}",
            String::from_utf8_lossy(&normalized.stderr)
        )
        .into());
    }
    Ok((root, version))
}

#[test]
fn sync_version_script_check_rejects_stale_cargo_lock_without_mutating_it(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = fixture_repo(&temp, "repo")?;
    let lock_path = repo.join("packages/codexy-runtime/Cargo.lock");
    let lock_text = fs::read_to_string(&lock_path)?;
    let selected_version = isolation::fixture_version(&repo)?;
    let stale_version = isolation::next_patch_version(&selected_version)?;
    let line_ending = if lock_text.contains("\r\n") { "\r\n" } else { "\n" };
    let stale_lock = lock_text.replacen(
        &format!("name = \"codexy-runtime\"{line_ending}version = \"{selected_version}\""),
        &format!("name = \"codexy-runtime\"{line_ending}version = \"{stale_version}\""),
        1,
    );
    assert_ne!(lock_text.as_bytes(), stale_lock.as_bytes(), "lock fixture did not change");
    fs::write(&lock_path, stale_lock.as_bytes())?;

    let output = run_sync_script(&repo, &["--check"])?;
    assert!(
        !output.status.success(),
        "sync-version --check unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let after = fs::read(&lock_path)?;
    assert_eq!(after, stale_lock.as_bytes(), "sync-version --check changed the stale Cargo.lock");

    Ok(())
}

#[test]
fn version_advance_requires_selected_public_identities_before_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = fixture_repo(&temp, "pre-activation")?;
    let before = isolation::version_surface_contents(&repo)?;
    let selected_version = isolation::fixture_version(&repo)?;
    let target = isolation::next_patch_version(&selected_version)?;
    let output = run_sync(&repo, &["--version", &target])?;
    assert!(!output.status.success(), "pre-activation version advance unexpectedly succeeded");
    assert_eq!(isolation::version_surface_contents(&repo)?, before);
    Ok(())
}
