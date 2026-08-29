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
    Ok(())
}

#[test]
fn sync_version_cli_rejects_stale_readme_pins_without_mutation()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let (root, version) = selected_fixture(&temp, "stale-readme")?;
    for relative in ["README.md", "README.ko.md"] {
        let path = root.join(relative);
        let stale = fs::read_to_string(&path)?.replace(
            &format!("--ref v{version}"),
            "--ref v0.0.1",
        );
        fs::write(path, stale)?;
    }
    let normalized = run_sync(&root, &["--version", &version])?;
    assert!(
        normalized.status.success(),
        "README normalization failed: {}",
        String::from_utf8_lossy(&normalized.stderr)
    );
    for relative in ["README.md", "README.ko.md"] {
        let readme = fs::read_to_string(root.join(relative))?;
        assert!(readme.contains(&format!("--ref v{version}")));
    }
    for relative in ["README.md", "README.ko.md"] {
        let path = root.join(relative);
        let original = fs::read(&path)?;
        let stale = String::from_utf8(original.clone())?
            .replace(&format!("--ref v{version}"), "--ref v0.0.1");
        assert_ne!(
            stale.as_bytes(),
            original.as_slice(),
            "fixture pin was not found"
        );
        fs::write(&path, stale.as_bytes())?;

        let output = run_sync(&root, &["--check"])?;
        assert!(
            !output.status.success(),
            "stale {relative} pin unexpectedly passed"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(relative),
            "stale {relative} diagnostic omitted the path: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&path)?, stale.as_bytes());
        fs::write(&path, original)?;
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
    let manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let version = manifest["version"]
        .as_str()
        .ok_or("manifest version")?
        .to_owned();
    let normalized = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", &version])
        .env("CODEXY_REPO_ROOT", &root)
        .current_dir(&root)
        .output()?;
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
    fs::copy(
        codexy_runtime::paths::repository_root().join("scripts/sync-plugin-version.sh"),
        repo.join("scripts/sync-plugin-version.sh"),
    )?;

    let lock_path = repo.join("packages/codexy-runtime/Cargo.lock");
    let lock_text = fs::read_to_string(&lock_path)?;
    let selected_version = isolation::fixture_version(&repo)?;
    let stale_version = isolation::next_patch_version(&selected_version)?;
    let stale_lock = stale_codexy_runtime_lock_version(&lock_text, &stale_version)?;
    assert_ne!(lock_text, stale_lock, "lock fixture did not change");
    fs::write(&lock_path, stale_lock)?;

    let output = FixtureCommand::new(repo.join("scripts/sync-plugin-version.sh"))
        .arg("--check")
        .current_dir(&repo)
        .output()?;
    assert!(
        !output.status.success(),
        "sync-version --check unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let after = fs::read_to_string(&lock_path)?;
    assert_eq!(
        stale_codexy_runtime_lock_version(&after, &stale_version)?,
        after,
        "sync-version --check changed the stale Cargo.lock"
    );

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
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", &target])
        .env("CODEXY_REPO_ROOT", &repo)
        .current_dir(&repo)
        .output()?;
    assert!(!output.status.success(), "pre-activation version advance unexpectedly succeeded");
    assert_eq!(isolation::version_surface_contents(&repo)?, before);
    Ok(())
}

fn stale_codexy_runtime_lock_version(
    lock_text: &str,
    stale_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut in_codexy_runtime = false;
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in lock_text.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_codexy_runtime = false;
        } else if trimmed == "name = \"codexy-runtime\"" {
            in_codexy_runtime = true;
        }

        if in_codexy_runtime && trimmed.starts_with("version = ") {
            lines.push(format!("version = \"{stale_version}\""));
            replaced = true;
            in_codexy_runtime = false;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        return Err("codexy-runtime package version not found in Cargo.lock".into());
    }
    Ok(format!("{}\n", lines.join("\n")))
}
