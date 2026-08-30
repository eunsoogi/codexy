use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard, OnceLock},
};

use super::isolation::{next_patch_version, version_surface_contents};

const README_COMMAND: &str = "codex plugin marketplace add eunsoogi/codexy";

#[test]
fn historical_readmes_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures = shared_fixtures()?;
    let root = &fixtures.selected_root;
    let selected = &fixtures.selected;
    let prior = previous_patch_version(selected)?;
    for (path, bytes) in version_surface_contents(root)? {
        fs::write(path, String::from_utf8(bytes)?.replace(selected, &prior))?;
    }
    for relative in ["README.md", "README.ko.md"] {
        let path = root.join(relative);
        let mutated = fs::read_to_string(&path)?.replace(
            &format!("{} --ref v{selected}", README_COMMAND),
            README_COMMAND,
        );
        fs::write(&path, mutated.as_bytes())?;
    }
    let expected = fixture_contents(root)?;
    let output = super::run_sync(root, &["--check"])?;
    assert!(!output.status.success());
    for (path, bytes) in expected {
        assert_eq!(fs::read(path)?, bytes);
    }
    Ok(())
}

#[test]
fn sync_version_script_rejects_malformed_readme_pins_without_mutation()
-> Result<(), Box<dyn std::error::Error>> {
    let fixtures = shared_fixtures()?;
    let selected_root = &fixtures.selected_root;
    let candidate_root = &fixtures.candidate_root;
    let selected = &fixtures.selected;
    let candidate = &fixtures.candidate;
    let prior = previous_patch_version(selected)?;
    for (case, candidate_mode) in [
        ("stale-prior", false),
        ("stale-candidate", true),
        ("missing", false),
        ("missing-v", false),
        ("invalid-semver", false),
        ("empty", false),
        ("duplicate", false),
    ] {
        for relative in ["README.md", "README.ko.md"] {
            let root = if candidate_mode { candidate_root } else { selected_root };
            let expected = if candidate_mode { candidate } else { selected };
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
            let output = super::run_sync(root, &[check])?;
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
            fs::write(&path, &original)?;
            assert_eq!(
                fs::read(&path)?,
                original,
                "{case} {relative} fixture bytes were not restored"
            );
        }
    }
    Ok(())
}

struct FixtureSeed {
    archive: Vec<u8>,
    selected: String,
    candidate: String,
}

impl FixtureSeed {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let (selected_root, selected) = super::selected_fixture(&temp, "readme-selected")?;
        let candidate_root = temp.path().join("readme-candidate");
        crate::support::copy_dir(&selected_root, &candidate_root)?;
        let candidate = next_patch_version(&selected)?;
        let prepared = super::run_sync(&candidate_root, &["--prepare-candidate", &candidate])?;
        if !prepared.status.success() {
            return Err(format!(
                "candidate preparation failed: {}",
                String::from_utf8_lossy(&prepared.stderr)
            )
            .into());
        }
        let archive = temp.path().join("readme-fixtures.tar");
        let archived = Command::new("tar")
            .args(["-cf"])
            .arg(&archive)
            .arg("-C")
            .arg(temp.path())
            .args(["readme-selected", "readme-candidate"])
            .status()?;
        if !archived.success() {
            return Err("README fixture archive failed".into());
        }
        Ok(Self {
            archive: fs::read(archive)?,
            selected,
            candidate,
        })
    }

    fn materialize(
        &'static self,
        serial: MutexGuard<'static, ()>,
    ) -> Result<SharedFixtures, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let archive = temp.path().join("readme-fixtures.tar");
        fs::write(&archive, &self.archive)?;
        let extracted = Command::new("tar")
            .args(["-xf"])
            .arg(&archive)
            .arg("-C")
            .arg(temp.path())
            .status()?;
        if !extracted.success() {
            return Err("README fixture extraction failed".into());
        }
        fs::remove_file(archive)?;
        Ok(SharedFixtures {
            selected_root: temp.path().join("readme-selected"),
            candidate_root: temp.path().join("readme-candidate"),
            selected: self.selected.clone(),
            candidate: self.candidate.clone(),
            _temp: temp,
            _serial: serial,
        })
    }
}

struct SharedFixtures {
    selected_root: PathBuf,
    candidate_root: PathBuf,
    selected: String,
    candidate: String,
    _temp: tempfile::TempDir,
    _serial: MutexGuard<'static, ()>,
}

fn shared_fixtures() -> Result<SharedFixtures, Box<dyn std::error::Error>> {
    static SEED: OnceLock<Result<FixtureSeed, String>> = OnceLock::new();
    static SERIAL: Mutex<()> = Mutex::new(());
    let serial = SERIAL
        .lock()
        .map_err(|_| "shared README fixture mutex poisoned")?;
    let seed = match SEED.get_or_init(|| FixtureSeed::create().map_err(|error| error.to_string())) {
        Ok(seed) => seed,
        Err(error) => return Err(error.clone().into()),
    };
    seed.materialize(serial)
}

fn fixture_contents(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, Box<dyn std::error::Error>> {
    let mut contents = version_surface_contents(root)?;
    for relative in ["README.md", "README.ko.md"] {
        let path = root.join(relative);
        contents.push((path.clone(), fs::read(path)?));
    }
    Ok(contents)
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
