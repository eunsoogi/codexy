use std::{fs, process::Command};

use super::{
    archive_repository, shared_repository_archive,
    isolation::version_surface_contents,
    strict_manifest::select_version_advance,
};

const COMPONENT_MANIFEST: &str = "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json";
const TARGET: &str = "1.3.1";

#[test]
fn markerless_version_mutation_rejects_strict_component_manifest_inputs_without_writes()
-> Result<(), Box<dyn std::error::Error>> {
    for (label, needle, replacement) in [
        (
            "top-level duplicate",
            "\"schema\": \"getcodexy.component-manifest.v1\",",
            "\"schema\": \"getcodexy.component-manifest.v1\", \"schema\": \"getcodexy.component-manifest.v1\",",
        ),
        (
            "nested duplicate",
            "\"pluginId\": \"codexy@codexy\",",
            "\"pluginId\": \"codexy@codexy\", \"pluginId\": \"codexy@codexy\",",
        ),
        (
            "oversized semver",
            "\"version\": \"1.3.0\"",
            "\"version\": \"2147483648.0.0\"",
        ),
        (
            "leading-zero semver",
            "\"version\": \"1.3.0\"",
            "\"version\": \"01.3.0\"",
        ),
        (
            "malformed semver",
            "\"version\": \"1.3.0\"",
            "\"version\": \"1.3\"",
        ),
        (
            "prerelease semver",
            "\"version\": \"1.3.0\"",
            "\"version\": \"1.3.0-beta\"",
        ),
        (
            "dependency-invalid compatible combination",
            "\"components\": [\"core\", \"github\"],",
            "\"components\": [\"github\"],",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let repo = archive_repository(shared_repository_archive()?, &temp, label)?;
        select_version_advance(&repo, TARGET)?;
        let manifest = repo.join(COMPONENT_MANIFEST);
        let text = fs::read_to_string(&manifest)?;
        let corrupted = text.replacen(needle, replacement, 1);
        assert_ne!(corrupted, text, "{label} fixture did not change");
        fs::write(&manifest, corrupted)?;
        let before = version_surface_contents(&repo)?;

        let output = sync_version(&repo, TARGET)?;
        assert!(
            !output.status.success(),
            "{label} unexpectedly completed mutation\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            version_surface_contents(&repo)?,
            before,
            "{label} changed a managed version surface before rejection"
        );
    }
    Ok(())
}

#[test]
fn markerless_version_mutation_rejects_late_cargo_rewriter_inputs_without_writes()
-> Result<(), Box<dyn std::error::Error>> {
    for (label, relative, needle, replacement) in [
        (
            "Cargo.toml alternate version spacing",
            "packages/codexy-runtime/Cargo.toml",
            "version = \"1.3.0\"",
            "version=\"1.3.0\"",
        ),
        (
            "Cargo.lock package field ordering",
            "packages/codexy-runtime/Cargo.lock",
            "name = \"codexy-runtime\"\nversion = \"1.3.0\"",
            "version = \"1.3.0\"\nname = \"codexy-runtime\"",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let repo = archive_repository(shared_repository_archive()?, &temp, label)?;
        select_version_advance(&repo, TARGET)?;
        let path = repo.join(relative);
        let text = fs::read_to_string(&path)?;
        let corrupted = text.replacen(needle, replacement, 1);
        assert_ne!(corrupted, text, "{label} fixture did not change");
        fs::write(&path, corrupted)?;
        let before = version_surface_contents(&repo)?;

        let output = sync_version(&repo, TARGET)?;
        assert!(
            !output.status.success(),
            "{label} unexpectedly completed mutation\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            version_surface_contents(&repo)?,
            before,
            "{label} changed a managed version surface before rejection"
        );
    }
    Ok(())
}

#[test]
fn markerless_version_mutation_accepts_the_component_semver_upper_bound_and_reads_back()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(shared_repository_archive()?, &temp, "max-bound")?;
    let target = "2147483647.0.0";
    select_version_advance(&repo, target)?;

    let output = sync_version(&repo, target)?;
    assert!(
        output.status.success(),
        "max-bound mutation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let check = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .arg("--check")
        .env("CODEXY_REPO_ROOT", &repo)
        .current_dir(&repo)
        .output()?;
    assert!(
        check.status.success(),
        "max-bound mutation did not read back\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    for (path, contents) in version_surface_contents(&repo)? {
        assert!(
            String::from_utf8_lossy(&contents).contains(target),
            "managed version surface did not contain the max-bound value: {}",
            path.display()
        );
    }
    Ok(())
}

fn sync_version(root: &std::path::Path, version: &str) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", version])
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()
}
