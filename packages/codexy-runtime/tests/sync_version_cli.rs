use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::OnceLock,
};

use crate::support::FixtureCommand;

#[path = "sync_version_cli/isolation.rs"]
mod isolation;
#[path = "sync_version_cli/admission.rs"]
mod admission;

#[test]
fn sync_version_cli_checks_manifest_marketplace_parity() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .arg("--check")
        .output()?;
    assert!(
        output.status.success(),
        "sync-version --check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("plugin version sync ok"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

#[test]
fn sync_version_cli_checks_release_tag_parity() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let version = manifest["version"].as_str().ok_or("manifest version")?;
    let matching_tag = format!("v{version}");
    let matching = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--check", "--tag", &matching_tag])
        .output()?;
    assert!(
        matching.status.success(),
        "matching release tag failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&matching.stdout),
        String::from_utf8_lossy(&matching.stderr)
    );

    let mismatched = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--check", "--tag", "1.1.0"])
        .output()?;
    assert!(
        !mismatched.status.success(),
        "tag without v prefix unexpectedly passed"
    );

    let stale = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--check", "--tag", "v9.9.9"])
        .output()?;
    assert!(
        !stale.status.success(),
        "mismatched release tag unexpectedly passed"
    );
    Ok(())
}

#[test]
fn sync_version_script_check_rejects_stale_cargo_lock_without_mutating_it(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = shared_repository_archive()?;
    let repo = archive_repository(archive, &temp, "repo")?;
    fs::copy(
        codexy_runtime::paths::repository_root().join("scripts/sync-plugin-version"),
        repo.join("scripts/sync-plugin-version"),
    )?;

    let lock_path = repo.join("packages/codexy-runtime/Cargo.lock");
    let lock_text = fs::read_to_string(&lock_path)?;
    let stale_lock = stale_codexy_runtime_lock_version(&lock_text, "9.9.9")?;
    assert_ne!(lock_text, stale_lock, "lock fixture did not change");
    fs::write(&lock_path, stale_lock)?;

    let output = FixtureCommand::new(repo.join("scripts/sync-plugin-version"))
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
        stale_codexy_runtime_lock_version(&after, "9.9.9")?,
        after,
        "sync-version --check changed the stale Cargo.lock"
    );

    Ok(())
}

#[test]
fn version_advance_requires_selected_public_identities_before_mutation(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = shared_repository_archive()?;
    let repo = archive_repository(archive, &temp, "pre-activation")?;
    let before = isolation::version_surface_contents(&repo)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", "1.3.1"])
        .env("CODEXY_REPO_ROOT", &repo)
        .current_dir(&repo)
        .output()?;
    assert!(!output.status.success(), "pre-activation version advance unexpectedly succeeded");
    assert_eq!(isolation::version_surface_contents(&repo)?, before);
    Ok(())
}

pub(super) struct RepositoryArchive {
    _temp: tempfile::TempDir,
    archive: PathBuf,
}

impl RepositoryArchive {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let archive = temp.path().join("repository.tar");
        let archive_status = Command::new("git")
            .args(["archive", "--format=tar", "HEAD"])
            .arg("-o")
            .arg(&archive)
            .current_dir(codexy_runtime::paths::repository_root())
            .status()?;
        if !archive_status.success() {
            return Err("git archive failed".into());
        }
        Ok(Self {
            _temp: temp,
            archive,
        })
    }
}

fn shared_repository_archive() -> Result<&'static RepositoryArchive, Box<dyn std::error::Error>> {
    static ARCHIVE: OnceLock<Result<RepositoryArchive, String>> = OnceLock::new();
    match ARCHIVE.get_or_init(|| RepositoryArchive::create().map_err(|error| error.to_string())) {
        Ok(archive) => Ok(archive),
        Err(error) => Err(error.clone().into()),
    }
}

pub(super) fn archive_repository(
    source: &RepositoryArchive,
    temp: &tempfile::TempDir,
    name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo = temp.path().join(name);
    fs::create_dir(&repo)?;
    let tar_status = Command::new("tar")
        .arg("-xf")
        .arg(&source.archive)
        .arg("-C")
        .arg(&repo)
        .status()?;
    assert!(tar_status.success(), "tar extract failed");
    for relative in [
        "packages/codexy-runtime/Cargo.toml",
        "packages/codexy-runtime/Cargo.lock",
        ".agents/plugins/release-publish-contract.json",
        "packages/getcodexy/pyproject.toml",
        "packages/codexy-runtime/src/version.rs",
        "packages/codexy-runtime/src/version/admission.rs",
        "packages/codexy-runtime/src/version/activation.rs",
        "packages/codexy-runtime/src/version/activation/receipt.rs",
        "packages/codexy-runtime/src/version/activation/receipt/fields.rs",
        "packages/codexy-runtime/src/version/bootstrap.rs",
        "packages/codexy-runtime/src/version/mutation.rs",
        "packages/codexy-runtime/src/version/wrappers.rs",
        "packages/codexy-runtime/tests/suites/all.rs",
        "plugins/codexy/hooks/capability-contract.json",
        "plugins/codexy/hooks/codexy_policy/filesystem_state.py",
        "plugins/codexy/skills/orchestration/references/workflow-profiles.json",
        "plugins/codexy/skills/orchestration/references/child-routing-policy.json",
        "plugins/codexy/skills/orchestration/references/routing-evaluation-corpus.json",
        "plugins/codexy/skills/orchestration/references/routing-evaluation-results.schema.json",
        "plugins/codexy/skills/orchestration/references/routing-evaluation-results.json",
        "plugins/codexy/skills/orchestration/SKILL.md",
        "plugins/codexy/skills/orchestration/agents/openai.yaml",
    ] {
        let destination = repo.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(
            codexy_runtime::paths::repository_root().join(relative),
            destination,
        )?;
    }
    Ok(repo)
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
