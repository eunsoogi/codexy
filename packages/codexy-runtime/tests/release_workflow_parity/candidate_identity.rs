use std::{fs, path::PathBuf, process::Output};

#[cfg(unix)]
use std::{os::unix::fs::PermissionsExt, process::Command};

use crate::support::FixtureCommand;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use super::{document, steps};

const SOURCE_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SOURCE_TREE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const STAGING_RUN_ID: u64 = 42;
const RUN_ATTEMPT: u64 = 3;

// Exercise the same validator that the candidate workflow runs before creating either receipt.
#[test]
fn selected_staging_identity_binds_receipt_provenance_and_archive_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let valid = StagingFixture::new()?.run(None)?;
    assert!(
        valid.status.success(),
        "valid selected artifact rejected: {}",
        String::from_utf8_lossy(&valid.stderr)
    );

    for (mutation, expected_error) in [
        ("source.tree", "source tree differs from the selected activation"),
        (
            "coreHandoff.manifest.sha256",
            "core handoff manifest differs from the selected activation",
        ),
        (
            "provenance.runAttempt",
            "receipt provenance differs from the downloaded run provenance",
        ),
    ] {
        let rejected = StagingFixture::new()?.run(Some(mutation))?;
        assert!(
            !rejected.status.success(),
            "receipt-only mutation was accepted: {mutation}"
        );
        assert!(
            String::from_utf8_lossy(&rejected.stderr).contains(expected_error),
            "wrong rejection for {mutation}: {}",
            String::from_utf8_lossy(&rejected.stderr)
        );
    }
    Ok(())
}

// The activation checkout predates this verifier, so exercise the workflow's exact Git projection command.
#[cfg(unix)]
#[test]
fn candidate_workflow_loads_verifier_from_workflow_revision_after_checkout()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = StagingFixture::new()?;
    run_git(&fixture.root, &["init", "--quiet", "--initial-branch=main"])?;
    run_git(&fixture.root, &["config", "user.email", "codexy-test@example.invalid"])?;
    run_git(&fixture.root, &["config", "user.name", "codexy-test"])?;
    run_git(&fixture.root, &["add", "-A"])?;
    run_git(&fixture.root, &["commit", "--quiet", "-m", "frozen activation"])?;
    let activation = run_git(&fixture.root, &["rev-parse", "HEAD"])?;

    let helper = fixture.root.join("scripts/verify_selected_staging_identity.py");
    fs::create_dir_all(helper.parent().ok_or("verifier parent")?)?;
    fs::copy(
        codexy_runtime::paths::repository_root()
            .join("scripts/verify_selected_staging_identity.py"),
        &helper,
    )?;
    let mut permissions = fs::metadata(&helper)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&helper, permissions)?;
    run_git(&fixture.root, &["add", "scripts/verify_selected_staging_identity.py"])?;
    run_git(&fixture.root, &["commit", "--quiet", "-m", "authenticated workflow verifier"])?;
    let workflow_sha = run_git(&fixture.root, &["rev-parse", "HEAD"])?;
    run_git(&fixture.root, &["checkout", "--quiet", "--detach", activation.trim()])?;
    assert!(!helper.exists(), "activation checkout unexpectedly contains the verifier");

    let workflow = document("prepare-version-candidate.yml")?;
    let candidate_steps = steps(&workflow, "prepare-candidate")?;
    let identity = candidate_steps
        .iter()
        .find(|step| step["name"] == "Verify selected staging receipt against activation and package bytes")
        .ok_or("selected staging verifier step")?;
    let runner_temp = tempfile::tempdir()?;
    let output = Command::new("bash")
        .args(["--noprofile", "--norc", "-e", "-o", "pipefail", "-c"])
        .arg(identity["run"].as_str().ok_or("selected verifier command")?)
        .current_dir(&fixture.root)
        .env("WORKFLOW_SHA", workflow_sha.trim())
        .env("RUNNER_TEMP", runner_temp.path())
        .env("STAGING_SOURCE_COMMIT", SOURCE_COMMIT)
        .env("STAGING_RUN_ID", STAGING_RUN_ID.to_string())
        .env("TARGET_VERSION", "2.0.0")
        .output()?;
    assert!(
        output.status.success(),
        "candidate verifier projection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

struct StagingFixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    receipt: PathBuf,
    run: PathBuf,
    archive: PathBuf,
}

impl StagingFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().to_path_buf();
        let plugin_manifest = root.join("plugins/codexy-devtools/.codex-plugin/plugin.json");
        fs::create_dir_all(plugin_manifest.parent().ok_or("plugin manifest parent")?)?;
        fs::write(&plugin_manifest, br#"{"name":"codexy-devtools","version":"2.0.0"}"#)?;

        let archive = root.join("staging/codexy-marketplace-plugin.tar.gz");
        fs::create_dir_all(archive.parent().ok_or("archive parent")?)?;
        let packed = FixtureCommand::new("tar")
            .args(["-C"])
            .arg_path(&root)
            .args(["-czf"])
            .arg_path(&archive)
            .arg("plugins/codexy-devtools")
            .output()?;
        assert!(packed.status.success(), "{}", String::from_utf8_lossy(&packed.stderr));
        let archive_sha256 = format!("{:x}", Sha256::digest(fs::read(&archive)?));

        let candidate = json!({
            "source": {"commit": SOURCE_COMMIT, "tree": SOURCE_TREE},
            "artifact": {"stagingRunId": STAGING_RUN_ID, "stagingRunAttempt": RUN_ATTEMPT},
            "classes": {
                "coreHandoff": {
                    "manifest": {"path": "handoff-runtime.json", "sha256": "c".repeat(64)},
                    "platforms": {"linux-x86_64": {"sha256": "d".repeat(64)}},
                },
                "devtoolsMcp": {"platforms": {"linux-x86_64": {"sha256": "e".repeat(64)}}},
            },
        });
        let provenance = json!({
            "runId": STAGING_RUN_ID,
            "runAttempt": RUN_ATTEMPT,
            "workflowRef": "eunsoogi/codexy/.github/workflows/runtime-candidate.yml@refs/heads/main",
        });
        let selection = json!({
            "candidate": candidate,
            "artifact": {"sha256": archive_sha256},
        });
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::write(
            root.join(".agents/plugins/runtime-activation.json"),
            serde_json::to_vec(&selection)?,
        )?;

        let receipt = root.join("staging/runtime-staging-receipt.json");
        fs::write(
            &receipt,
            serde_json::to_vec(&json!({
                "candidate": selection["candidate"],
                "artifact": selection["artifact"],
                "provenance": provenance,
            }))?,
        )?;
        fs::write(
            root.join("staging/runtime-staging-provenance.json"),
            serde_json::to_vec(&provenance)?,
        )?;
        let run = root.join("staging/runtime-staging-run.json");
        fs::write(
            &run,
            serde_json::to_vec(&json!({"id": STAGING_RUN_ID, "run_attempt": RUN_ATTEMPT}))?,
        )?;

        Ok(Self {
            _temporary: temporary,
            root,
            receipt,
            run,
            archive,
        })
    }

    fn run(&self, mutation: Option<&str>) -> Result<Output, Box<dyn std::error::Error>> {
        if let Some(mutation) = mutation {
            let mut receipt: Value = serde_json::from_slice(&fs::read(&self.receipt)?)?;
            match mutation {
                "source.tree" => receipt["candidate"]["source"]["tree"] = json!("f".repeat(40)),
                "coreHandoff.manifest.sha256" => {
                    receipt["candidate"]["classes"]["coreHandoff"]["manifest"]["sha256"] =
                        json!("0".repeat(64));
                }
                "provenance.runAttempt" => receipt["provenance"]["runAttempt"] = json!(4),
                _ => return Err(format!("unknown staging mutation: {mutation}").into()),
            }
            fs::write(&self.receipt, serde_json::to_vec(&receipt)?)?;
        }

        let helper = codexy_runtime::paths::repository_root()
            .join("scripts/verify_selected_staging_identity.py");
        Ok(FixtureCommand::new(helper)
            .arg_path(&self.receipt)
            .arg_path(&self.run)
            .arg_path(&self.archive)
            .current_dir(&self.root)
            .env("STAGING_SOURCE_COMMIT", SOURCE_COMMIT)
            .env("STAGING_RUN_ID", STAGING_RUN_ID.to_string())
            .env("TARGET_VERSION", "2.0.0")
            .output()?)
    }
}

#[cfg(unix)]
fn run_git(cwd: &std::path::Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(cwd).args(args).output()?;
    if !output.status.success() {
        return Err(format!("git {args:?} failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}
