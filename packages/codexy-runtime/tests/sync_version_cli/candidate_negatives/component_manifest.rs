use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use serde_json::{Value, json};

use super::super::restoration::{ByteSnapshot, VERSION_FIXTURE_PATHS};
use super::{
    TempDir, TestResult, bootstrap_candidate_version, fixture_version, mutate_json,
    next_patch_version, reject, run,
};

const COMPONENT_MANIFEST: &str =
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json";

#[test]
fn candidate_preparation_preserves_the_packaged_component_manifest() -> TestResult {
    let fixture = candidate_fixture()?;
    let root = &fixture.root;
    let snapshot = ByteSnapshot::capture(root, VERSION_FIXTURE_PATHS)?;
    let mut restoration = snapshot.guard();

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(root.join(COMPONENT_MANIFEST))?)?;
    let selected_version = fixture_version(&root)?;
    let candidate_version = next_patch_version(&selected_version)?;
    for field in ["components", "compatibleCombinations"] {
        for entry in manifest[field]
            .as_array()
            .ok_or("component manifest array")?
        {
            assert_eq!(
                entry["version"], selected_version,
                "candidate {field} changed selected identity"
            );
        }
    }
    let contract: Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    assert_eq!(contract["bootstrap"]["selectedVersion"], selected_version);
    assert_eq!(contract["bootstrap"]["candidateVersion"], candidate_version);
    restoration.restore_checked()?;
    Ok(())
}

#[test]
fn candidate_check_rejects_each_component_manifest_drift() -> TestResult {
    let fixture = candidate_fixture()?;
    let root = &fixture.root;
    let snapshot = ByteSnapshot::capture(root, VERSION_FIXTURE_PATHS)?;
    for field in ["components", "compatibleCombinations"] {
        let mut restoration = snapshot.guard();
        let candidate_version = next_patch_version(&fixture_version(&root)?)?;
        mutate_json(&root.join(COMPONENT_MANIFEST), |value| {
            value[field][0]["version"] = json!(candidate_version);
        })?;
        reject(
            &root,
            &["--check-candidate"],
            &format!("{field}-component-drift"),
        )?;
        restoration.restore_checked()?;
    }
    Ok(())
}

pub(super) struct Fixture {
    pub(super) root: PathBuf,
    _temp: TempDir,
}

struct FixtureSeed {
    _temp: TempDir,
    selected: PathBuf,
    candidate: PathBuf,
}

impl FixtureSeed {
    fn create() -> TestResult<Self> {
        let temp = tempfile::tempdir()?;
        let (root, selected_version) = super::super::selected_fixture(&temp, "selected-seed")?;
        let bootstrap = root.join("packages/codexy-runtime/src/version/bootstrap.rs");
        let text = fs::read_to_string(&bootstrap)?;
        let current_candidate = bootstrap_candidate_version(&root)?;
        fs::write(
            bootstrap,
            text.replace(
                &format!("CANDIDATE_VERSION: &str = \"{current_candidate}\""),
                &format!("CANDIDATE_VERSION: &str = \"{selected_version}\""),
            ),
        )?;
        mutate_json(
            &root.join(".agents/plugins/release-publish-contract.json"),
            |value| value["bootstrap"]["candidateVersion"] = json!(selected_version),
        )?;
        let selected = archive_fixture(&root, temp.path().join("selected.tar"))?;
        let candidate_root = temp.path().join("candidate-seed");
        crate::support::copy_dir(&root, &candidate_root)?;
        let candidate_version = next_patch_version(&selected_version)?;
        let prepared = run(
            &candidate_root,
            &["--prepare-candidate", &candidate_version],
        )?;
        assert!(
            prepared.status.success(),
            "candidate preparation failed: {}",
            String::from_utf8_lossy(&prepared.stderr)
        );
        let candidate = archive_fixture(&candidate_root, temp.path().join("candidate.tar"))?;
        fs::remove_dir_all(root)?;
        fs::remove_dir_all(candidate_root)?;
        Ok(Self {
            _temp: temp,
            selected,
            candidate,
        })
    }

    fn materialize(&self, archive: &Path) -> TestResult<Fixture> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("repo");
        fs::create_dir(&root)?;
        let extracted = Command::new("tar")
            .arg("-xf")
            .arg(archive)
            .arg("-C")
            .arg(&root)
            .status()?;
        assert!(extracted.success(), "candidate fixture extraction failed");
        Ok(Fixture { root, _temp: temp })
    }
}

fn archive_fixture(root: &Path, archive: PathBuf) -> TestResult<PathBuf> {
    let status = Command::new("tar")
        .arg("-cf")
        .arg(&archive)
        .arg("-C")
        .arg(root)
        .arg(".")
        .status()?;
    assert!(status.success(), "candidate fixture archive failed");
    Ok(archive)
}

fn fixture_seed() -> TestResult<&'static FixtureSeed> {
    static SEED: OnceLock<Result<FixtureSeed, String>> = OnceLock::new();
    match SEED.get_or_init(|| FixtureSeed::create().map_err(|error| error.to_string())) {
        Ok(seed) => Ok(seed),
        Err(error) => Err(error.clone().into()),
    }
}

pub(super) fn selected_fixture() -> TestResult<Fixture> {
    let seed = fixture_seed()?;
    seed.materialize(&seed.selected)
}

pub(super) fn candidate_fixture() -> TestResult<Fixture> {
    let seed = fixture_seed()?;
    seed.materialize(&seed.candidate)
}
