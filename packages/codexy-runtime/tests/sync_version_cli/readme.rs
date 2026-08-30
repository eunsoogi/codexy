use std::fs;

use super::isolation::{next_patch_version, version_surface_contents};

const README_COMMAND: &str = "codex plugin marketplace add eunsoogi/codexy";

#[test]
fn historical_readmes_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let (root, selected) = super::selected_fixture(&temp, "historical-readme")?;
    let prior = previous_patch_version(&selected)?;
    for (path, bytes) in version_surface_contents(&root)? {
        fs::write(path, String::from_utf8(bytes)?.replace(&selected, &prior))?;
    }
    let mut expected = Vec::new();
    for relative in ["README.md", "README.ko.md"] {
        let path = root.join(relative);
        let mutated = fs::read_to_string(&path)?.replace(
            &format!("{} --ref v{selected}", README_COMMAND),
            README_COMMAND,
        );
        fs::write(&path, mutated.as_bytes())?;
        expected.push((path, mutated.into_bytes()));
    }
    let output = super::run_sync_script(&root, &["--check"])?;
    assert!(!output.status.success());
    for (path, bytes) in expected {
        assert_eq!(fs::read(path)?, bytes);
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
            let (root, selected) =
                super::selected_fixture(&temp, &format!("readme-{case}-{index}"))?;
            let prior = previous_patch_version(&selected)?;
            let candidate = next_patch_version(&selected)?;
            if candidate_mode {
                let prepared = super::run_sync_script(&root, &["--prepare-candidate", &candidate])?;
                assert!(
                    prepared.status.success(),
                    "candidate preparation failed: {}",
                    String::from_utf8_lossy(&prepared.stderr)
                );
            }
            let expected = if candidate_mode {
                &candidate
            } else {
                &selected
            };
            let direct_pin = format!("{README_COMMAND} --ref v{expected}");
            let (replacement, diagnostic) = match case {
                "stale-prior" => (
                    format!("{README_COMMAND} --ref v{prior}"),
                    "version mismatch",
                ),
                "stale-candidate" => (
                    format!("{README_COMMAND} --ref v{selected}"),
                    "version mismatch",
                ),
                "missing" => (
                    README_COMMAND.to_owned(),
                    "exactly one direct marketplace pin",
                ),
                "missing-v" => (
                    format!("{README_COMMAND} --ref {expected}"),
                    "must start with v",
                ),
                "invalid-semver" => (
                    format!("{README_COMMAND} --ref vnot-a-version"),
                    "semver-like MAJOR.MINOR.PATCH",
                ),
                "empty" => (
                    format!("{README_COMMAND} --ref "),
                    "marketplace pin is empty",
                ),
                "duplicate" => (
                    format!("{direct_pin}\n{direct_pin}"),
                    "exactly one direct marketplace pin",
                ),
                other => return Err(format!("unknown README case: {other}").into()),
            };
            let path = root.join(relative);
            let original = fs::read(&path)?;
            let mutated = String::from_utf8(original.clone())?.replace(&direct_pin, &replacement);
            assert_ne!(
                mutated.as_bytes(),
                original.as_slice(),
                "fixture pin was not found"
            );
            fs::write(&path, mutated.as_bytes())?;
            let check = if candidate_mode {
                "--check-candidate"
            } else {
                "--check"
            };
            let output = super::run_sync_script(&root, &[check])?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                !output.status.success(),
                "{case} {relative} unexpectedly passed"
            );
            assert!(
                stderr.contains(relative),
                "{case} diagnostic omitted {relative}: {stderr}"
            );
            assert!(
                stderr.contains(diagnostic),
                "{case} diagnostic changed: {stderr}"
            );
            assert_eq!(fs::read(&path)?, mutated.as_bytes(), "README was mutated");
        }
    }
    Ok(())
}

fn previous_patch_version(version: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (prefix, patch) = version.rsplit_once('.').ok_or("version")?;
    Ok(format!(
        "{prefix}.{}",
        patch
            .parse::<u64>()?
            .checked_sub(1)
            .ok_or("version underflow")?
    ))
}
