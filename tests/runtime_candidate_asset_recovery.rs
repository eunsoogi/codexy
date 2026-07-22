use std::{fs, os::unix::fs::PermissionsExt as _, path::{Path, PathBuf}, process::Command};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

#[test]
fn existing_assets_stabilize_retry_bytes_with_a_success_binding() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    fixture.publish_all()?;
    fixture.write_local_receipt(200, 3)?;
    let before = fixture.remote_bytes()?;
    let output = fixture.run_with(200, 3)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let after = fixture.remote_bytes()?;
    assert!(
        before
            .iter()
            .all(|entry| after.iter().any(|candidate| candidate == entry))
    );
    let binding: Value = serde_json::from_slice(&fs::read(
        fixture.remote.join("runtime-candidate-publication-200-3.json"),
    )?)?;
    assert_eq!(binding["workflow"]["runId"], 200);
    let receipt = before.iter().find(|(name, _)| name == "runtime-candidate-receipt.json").ok_or("published receipt")?;
    assert_eq!(fs::read(fixture.dist.join("runtime-candidate-receipt.json"))?, receipt.1);
    assert_eq!(
        fixture.upload_log()?,
        "runtime-candidate-publication-200-3.json\n"
    );
    Ok(())
}

#[test]
fn missing_provenance_is_uploaded_from_the_existing_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    fixture.publish_archive_and_receipt()?;
    fixture.write_local_receipt(200, 3)?;
    let output = fixture.run_with(200, 3)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let published: Value = serde_json::from_str(&fs::read_to_string(
        fixture.remote.join("runtime-candidate-provenance.json"),
    )?)?;
    assert_eq!(published["runId"], 100);
    assert_eq!(
        fixture.upload_log()?,
        "runtime-candidate-provenance.json\nruntime-candidate-publication-200-3.json\n"
    );
    Ok(())
}

#[test]
fn mismatched_existing_asset_fails_without_upload_or_clobber() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    fixture.publish_all()?;
    fs::write(fixture.remote.join("codexy-marketplace-plugin.tar.gz"), b"mismatch")?;
    let before = fixture.remote_bytes()?;
    let output = fixture.run_with(100, 1)?;
    assert!(!output.status.success());
    assert_eq!(fixture.remote_bytes()?, before);
    assert!(fixture.upload_log()?.is_empty());
    Ok(())
}

#[test]
fn failed_attempt_bindings_remain_append_only_for_a_later_successful_retry()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    fixture.publish_all()?;
    let first = fixture.run_with(100, 1)?;
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let first_binding = fixture.remote.join("runtime-candidate-publication-100-1.json");
    let first_bytes = fs::read(&first_binding)?;
    fixture.write_local_receipt(200, 2)?;
    let retry = fixture.run_with(200, 2)?;
    assert!(retry.status.success(), "{}", String::from_utf8_lossy(&retry.stderr));
    assert_eq!(fs::read(&first_binding)?, first_bytes);
    let retry_binding: Value = serde_json::from_slice(&fs::read(
        fixture.remote.join("runtime-candidate-publication-200-2.json"),
    )?)?;
    assert_eq!(retry_binding["workflow"]["runId"], 200);
    assert_eq!(retry_binding["workflow"]["runAttempt"], 2);
    Ok(())
}

struct Fixture {
    _temp: tempfile::TempDir,
    dist: PathBuf,
    remote: PathBuf,
    bin: PathBuf,
    log: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let dist = temp.path().join("dist");
        let remote = temp.path().join("remote");
        let bin = temp.path().join("bin");
        fs::create_dir_all(dist.join("candidate/plugins/codexy"))?;
        fs::create_dir_all(&remote)?;
        fs::create_dir_all(&bin)?;
        fs::write(dist.join("codexy-marketplace-plugin.tar.gz"), b"stable archive")?;
        fs::write(
            dist.join("candidate/plugins/codexy/runtime-candidate.json"),
            serde_json::to_vec(&json!({"artifact":{"tag":"runtime-candidate-test"},"schema":"codexy-runtime-candidate/v1","source":{"commit":"abc"}}))?,
        )?;
        let log = temp.path().join("uploads.log");
        fs::write(&log, "")?;
        write_fake_gh(&bin.join("gh"))?;
        let fixture = Self { _temp: temp, dist, remote, bin, log };
        fixture.write_local_receipt(100, 1)?;
        Ok(fixture)
    }

    fn write_local_receipt(&self, run_id: u64, attempt: u64) -> Result<(), Box<dyn std::error::Error>> {
        let candidate: Value = serde_json::from_slice(&fs::read(self.dist.join("candidate/plugins/codexy/runtime-candidate.json"))?)?;
        let archive_sha = format!("{:x}", Sha256::digest(fs::read(self.dist.join("codexy-marketplace-plugin.tar.gz"))?));
        let manifest_sha = format!("{:x}", Sha256::digest(serde_json::to_vec(&candidate)?));
        let receipt = json!({
            "schema":"codexy-runtime-candidate-receipt/v1",
            "candidate":candidate,
            "artifact":{"url":"https://github.com/eunsoogi/codexy/releases/download/runtime-candidate-test/codexy-marketplace-plugin.tar.gz","sha256":archive_sha,"payloadManifestSha256":manifest_sha},
            "provenance":{"repositoryId":1269350143,"runAttempt":attempt,"runId":run_id,"workflowPath":".github/workflows/runtime-candidate.yml","workflowRunUrl":format!("https://github.com/eunsoogi/codexy/actions/runs/{run_id}")}
        });
        fs::write(self.dist.join("runtime-candidate-receipt.json"), serde_json::to_vec(&receipt)?)?;
        fs::write(self.dist.join("runtime-candidate-provenance.json"), serde_json::to_vec(&receipt["provenance"])?)?;
        Ok(())
    }

    fn publish_archive_and_receipt(&self) -> Result<(), Box<dyn std::error::Error>> {
        for name in ["codexy-marketplace-plugin.tar.gz", "runtime-candidate-receipt.json"] {
            fs::copy(self.dist.join(name), self.remote.join(name))?;
        }
        Ok(())
    }

    fn publish_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.publish_archive_and_receipt()?;
        fs::copy(self.dist.join("runtime-candidate-provenance.json"), self.remote.join("runtime-candidate-provenance.json"))?;
        Ok(())
    }

    fn run_with(&self, run_id: u64, attempt: u64) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        let path = format!("{}:{}", self.bin.display(), std::env::var("PATH")?);
        Ok(Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/reconcile-runtime-candidate-assets"))
            .arg(&self.dist)
            .env("PATH", path)
            .env("CANDIDATE_TAG", "runtime-candidate-test")
            .env("GITHUB_RUN_ID", run_id.to_string())
            .env("GITHUB_RUN_ATTEMPT", attempt.to_string())
            .env("GITHUB_SERVER_URL", "https://github.com")
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("FAKE_RELEASE_DIR", &self.remote)
            .env("FAKE_UPLOAD_LOG", &self.log)
            .output()?)
    }

    fn remote_bytes(&self) -> Result<Vec<(String, Vec<u8>)>, std::io::Error> {
        let mut entries = fs::read_dir(&self.remote)?.map(|entry| {
            let path = entry?.path();
            Ok((path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_owned(), fs::read(path)?))
        }).collect::<Result<Vec<_>, std::io::Error>>()?;
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(entries)
    }

    fn upload_log(&self) -> Result<String, std::io::Error> { fs::read_to_string(&self.log) }
}

fn write_fake_gh(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, r##"#!/bin/sh
set -eu
shift
case "$1" in
  view) find "$FAKE_RELEASE_DIR" -type f -maxdepth 1 -exec basename {} \; | sort ;;
  download) while test "$1" != "--pattern"; do shift; done; name="$2"; shift 2; test "$1" = "--output"; cp "$FAKE_RELEASE_DIR/$name" "$2" ;;
  upload) name="$(basename "$3")"; cp "$3" "$FAKE_RELEASE_DIR/$name"; printf '%s\n' "$name" >> "$FAKE_UPLOAD_LOG" ;;
  *) exit 64 ;;
esac
"##)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
