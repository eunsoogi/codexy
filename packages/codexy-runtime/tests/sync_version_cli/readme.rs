use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use super::isolation::{next_patch_version, version_surface_contents};

const README_COMMAND: &str = "codex plugin marketplace add eunsoogi/codexy";

#[test]
fn historical_readmes_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures = shared_fixtures()?;
    let root = &fixtures.root;
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
    let root = &fixtures.root;
    let selected = &fixtures.selected;
    let prior = previous_patch_version(selected)?;
    for (case, replacement, diagnostic) in [
        (
            "stale-prior",
            format!("{README_COMMAND} --ref v{prior}"),
            "version mismatch",
        ),
        (
            "missing",
            README_COMMAND.to_owned(),
            "exactly one direct marketplace pin",
        ),
        (
            "missing-v",
            format!("{README_COMMAND} --ref {selected}"),
            "must start with v",
        ),
        (
            "invalid-semver",
            format!("{README_COMMAND} --ref vnot-a-version"),
            "semver-like MAJOR.MINOR.PATCH",
        ),
        (
            "empty",
            format!("{README_COMMAND} --ref "),
            "marketplace pin is empty",
        ),
        (
            "duplicate",
            format!("{README_COMMAND} --ref v{selected}\n{README_COMMAND} --ref v{selected}"),
            "exactly one direct marketplace pin",
        ),
    ] {
        for relative in ["README.md", "README.ko.md"] {
            assert_malformed(root, selected, relative, case, &replacement, diagnostic, "--check")?;
        }
    }
    let candidate = next_patch_version(selected)?;
    let prepared = super::run_sync(root, &["--prepare-candidate", &candidate])?;
    assert!(
        prepared.status.success(),
        "candidate preparation failed: {}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let stale_selected = format!("{README_COMMAND} --ref v{selected}");
    for relative in ["README.md", "README.ko.md"] {
        assert_malformed(
            root,
            &candidate,
            relative,
            "stale-candidate",
            &stale_selected,
            "version mismatch",
            "--check-candidate",
        )?;
    }
    Ok(())
}

fn assert_malformed(
    root: &Path,
    expected: &str,
    relative: &str,
    case: &str,
    replacement: &str,
    diagnostic: &str,
    check: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join(relative);
    let original = fs::read(&path)?;
    let direct_pin = format!("{README_COMMAND} --ref v{expected}");
    let mutated = String::from_utf8(original.clone())?.replace(&direct_pin, replacement);
    assert_ne!(mutated.as_bytes(), original, "fixture pin was not found");
    fs::write(&path, mutated.as_bytes())?;
    let output = super::run_sync(root, &[check])?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{case} {relative} unexpectedly passed");
    assert!(stderr.contains(relative), "{case} diagnostic omitted {relative}: {stderr}");
    assert!(stderr.contains(diagnostic), "{case} diagnostic changed: {stderr}");
    assert_eq!(fs::read(&path)?, mutated.as_bytes(), "README was mutated");
    fs::write(&path, &original)?;
    assert_eq!(fs::read(&path)?, original, "{case} {relative} fixture bytes were not restored");
    Ok(())
}

struct FixtureSeed {
    archive: Vec<u8>,
    selected: String,
}

impl FixtureSeed {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let (_root, selected) = super::selected_fixture_snapshot(&temp, "readme-root")?;
        let archive = temp.path().join("readme-fixtures.tar");
        let archived = Command::new("tar")
            .args(["-cf"])
            .arg(&archive)
            .arg("-C")
            .arg(temp.path())
            .arg("readme-root")
            .status()?;
        if !archived.success() {
            return Err("README fixture archive failed".into());
        }
        Ok(Self {
            archive: fs::read(archive)?,
            selected,
        })
    }

    fn materialize(&self) -> Result<SharedFixtures, Box<dyn std::error::Error>> {
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
            root: temp.path().join("readme-root"),
            selected: self.selected.clone(),
            _temp: temp,
        })
    }
}

struct SharedFixtures {
    root: PathBuf,
    selected: String,
    _temp: tempfile::TempDir,
}

fn shared_fixtures() -> Result<SharedFixtures, Box<dyn std::error::Error>> {
    static SEED: OnceLock<Result<FixtureSeed, String>> = OnceLock::new();
    let seed = match SEED.get_or_init(|| FixtureSeed::create().map_err(|error| error.to_string())) {
        Ok(seed) => seed,
        Err(error) => return Err(error.clone().into()),
    };
    seed.materialize()
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
