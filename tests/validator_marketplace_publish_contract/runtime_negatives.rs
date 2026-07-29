use std::{fs, path::PathBuf, process::Command};

use serde_json::{Value, json};

#[test]
fn validator_rejects_former_candidate_release_and_wrong_staging_contracts()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = SnapshotFixture::new()?;
    let path = fixture.root.join(".agents/plugins/release-publish-contract.json");
    let baseline: Value = serde_json::from_slice(&fs::read(&path)?)?;
    let mut cases = Vec::new();

    let mut old_key = baseline.clone();
    old_key["runtime"]["candidateTagPrefix"] = json!("runtime-candidate-");
    cases.push(("former candidate key", old_key));
    for (label, field, value) in [
        ("candidate tag", "selectedTag", "runtime-candidate-1.3.0"),
        ("staging workflow", "stagingWorkflow", ".github/workflows/old.yml"),
        ("activation workflow", "activationWorkflow", ".github/workflows/old.yml"),
        ("final publisher", "finalPublisherWorkflow", ".github/workflows/old.yml"),
    ] {
        let mut changed = baseline.clone();
        changed["runtime"][field] = json!(value);
        cases.push((label, changed));
    }
    let mut retention = baseline.clone();
    retention["runtime"]["artifactRetentionDays"] = json!(90);
    cases.push(("retention", retention));

    for (label, changed) in cases {
        fs::write(&path, serde_json::to_vec_pretty(&changed)?)?;
        let output = validate(&fixture.root)?;
        assert!(
            !output.status.success(),
            "validator accepted invalid {label} contract"
        );
    }
    Ok(())
}

fn validate(root: &std::path::Path) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--check")
        .env("CODEXY_REPO_ROOT", root)
        .output()
}

struct SnapshotFixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
}

impl SnapshotFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let archive = temporary.path().join("repo.tar");
        assert!(
            Command::new("git")
                .args(["archive", "--format=tar", "HEAD", "-o"])
                .arg(&archive)
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .status()?
                .success()
        );
        let root = temporary.path().join("repo");
        fs::create_dir(&root)?;
        assert!(
            Command::new("tar")
                .args(["-xf"])
                .arg(&archive)
                .arg("-C")
                .arg(&root)
                .status()?
                .success()
        );
        Ok(Self {
            _temporary: temporary,
            root,
        })
    }
}
