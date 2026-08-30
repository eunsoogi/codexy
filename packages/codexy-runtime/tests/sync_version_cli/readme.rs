use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, OnceLock},
};

use super::isolation::{next_patch_version, version_surface_contents};

const README_COMMAND: &str = "codex plugin marketplace add eunsoogi/codexy";

#[test]
fn historical_readmes_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures = shared_fixtures()?;
    let root = &fixtures.selected_root;
    let selected = &fixtures.selected;
    let mut restoration = Restoration::capture(&[root])?;
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
    restoration.restore()?;
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
    let mut restoration = Restoration::capture(&[selected_root, candidate_root])?;
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
    restoration.restore()?;
    Ok(())
}

struct SharedFixtures {
    _temp: tempfile::TempDir,
    selected_root: PathBuf,
    candidate_root: PathBuf,
    selected: String,
    candidate: String,
}

impl SharedFixtures {
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
        Ok(Self {
            _temp: temp,
            selected_root,
            candidate_root,
            selected,
            candidate,
        })
    }
}

fn shared_fixtures() -> Result<MutexGuard<'static, SharedFixtures>, Box<dyn std::error::Error>> {
    static FIXTURES: OnceLock<Result<Mutex<SharedFixtures>, String>> = OnceLock::new();
    let fixtures = match FIXTURES.get_or_init(|| {
        SharedFixtures::create()
            .map(Mutex::new)
            .map_err(|error| error.to_string())
    }) {
        Ok(fixtures) => fixtures,
        Err(error) => return Err(error.clone().into()),
    };
    fixtures
        .lock()
        .map_err(|_| "shared README fixture mutex poisoned".into())
}

struct Restoration {
    originals: Vec<(PathBuf, Vec<u8>)>,
}

impl Restoration {
    fn capture(roots: &[&PathBuf]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut originals = Vec::new();
        for root in roots {
            originals.extend(fixture_contents(root)?);
        }
        Ok(Self { originals })
    }

    fn restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (path, bytes) in &self.originals {
            fs::write(path, bytes)?;
        }
        for (path, bytes) in &self.originals {
            assert_eq!(fs::read(path)?, *bytes, "fixture bytes were not restored");
        }
        self.originals.clear();
        Ok(())
    }
}

impl Drop for Restoration {
    fn drop(&mut self) {
        for (path, bytes) in &self.originals {
            let _ = fs::write(path, bytes);
        }
    }
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
