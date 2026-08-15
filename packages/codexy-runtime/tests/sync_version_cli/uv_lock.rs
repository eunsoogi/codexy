use std::fs;

use crate::support::FixtureCommand;

use super::{archive_repository, shared_repository_archive};

#[test]
fn sync_version_script_rejects_a_pyproject_projection_that_differs_from_uv_lock()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = shared_repository_archive()?;
    let repo = archive_repository(archive, &temp, "repo")?;
    fs::copy(
        codexy_runtime::paths::repository_root().join("scripts/sync-plugin-version.sh"),
        repo.join("scripts/sync-plugin-version.sh"),
    )?;

    let package_path = repo.join("packages/getcodexy/pyproject.toml");
    let package_text = fs::read_to_string(&package_path)?;
    let current_version = package_text
        .lines()
        .find_map(|line| line.trim().strip_prefix("version = \"")?.strip_suffix('"'))
        .ok_or("getcodexy package version")?;
    let stale_package = package_text.replacen(
        &format!("version = \"{current_version}\""),
        "version = \"9.9.9\"",
        1,
    );
    assert_ne!(package_text, stale_package, "package fixture did not change");
    fs::write(&package_path, &stale_package)?;

    let output = FixtureCommand::new(repo.join("scripts/sync-plugin-version.sh"))
        .arg("--check")
        .current_dir(&repo)
        .output()?;
    assert!(
        !output.status.success(),
        "sync-version --check unexpectedly accepted a stale pyproject projection\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&package_path)?, stale_package);
    Ok(())
}
