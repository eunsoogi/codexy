use std::{fs, path::PathBuf, process::Output};

use crate::support::FixtureCommand;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

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
