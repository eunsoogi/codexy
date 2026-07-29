use std::{
    fs,
    path::{Path, PathBuf},
    process::Output,
};

use crate::support::{self, FixtureCommand as Command};

const REPOSITORY_ID: &str = "1269350143";
const RUN_ID: &str = "42";
const SOURCE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn downloads_one_authenticated_unexpired_staging_artifact()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(&run_json(SOURCE_COMMIT), &artifacts_json(false, 1), true)?;
    assert!(
        output.status.success(),
        "authenticated staging download failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture.temp.path().join("staging/runtime-staging-receipt.json").is_file());
    assert!(fixture.temp.path().join("staging/runtime-staging-run.json").is_file());
    Ok(())
}

#[test]
fn rejects_mismatched_staging_identity() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(
        &run_json("ffffffffffffffffffffffffffffffffffffffff"),
        &artifacts_json(false, 1),
        true,
    )?;
    assert_failure(&output, "staging run source commit mismatch");
    Ok(())
}

#[test]
fn rejects_expired_staging_artifact() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(&run_json(SOURCE_COMMIT), &artifacts_json(true, 1), true)?;
    assert_failure(&output, "staging artifact expired");
    Ok(())
}

#[test]
fn rejects_duplicate_staging_artifact_identity() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(&run_json(SOURCE_COMMIT), &artifacts_json(false, 2), true)?;
    assert_failure(&output, "staging artifact identity is not unique");
    Ok(())
}

#[test]
fn rejects_unauthenticated_staging_download() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(&run_json(SOURCE_COMMIT), &artifacts_json(false, 1), false)?;
    assert_failure(&output, "authenticated GitHub token is required");
    Ok(())
}

struct Fixture {
    temp: tempfile::TempDir,
    gh: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let gh = temp.path().join("gh");
        fs::write(
            &gh,
            "#!/bin/sh\ncase \"$*\" in\n  *'/actions/artifacts/'*'/zip') cat \"$FAKE_ZIP\" ;;\n  *'/artifacts') cat \"$FAKE_ARTIFACTS\" ;;\n  *'/actions/runs/'*) cat \"$FAKE_RUN\" ;;\n  *) exit 91 ;;\nesac\n",
        )?;
        support::make_executable(&gh)?;
        let archive = temp.path().join("fixture-artifact.zip");
        let status = Command::new("python3")
            .args([
                "-c",
                "import sys,zipfile; z=zipfile.ZipFile(sys.argv[1],'w'); z.writestr('runtime-staging-receipt.json','{}'); z.close()",
            ])
            .arg(&archive)
            .status()?;
        assert!(status.success(), "failed to create staging zip fixture");
        Ok(Self { temp, gh })
    }

    fn run(
        &self,
        run: &str,
        artifacts: &str,
        authenticated: bool,
    ) -> Result<Output, Box<dyn std::error::Error>> {
        let run_path = self.temp.path().join("run.json");
        let artifacts_path = self.temp.path().join("artifacts.json");
        fs::write(&run_path, run)?;
        fs::write(&artifacts_path, artifacts)?;
        let path = format!(
            "{}:{}",
            self.gh.parent().ok_or("fake gh parent")?.display(),
            std::env::var("PATH")?
        );
        let mut command = Command::new(script());
        command
            .arg(self.temp.path().join("staging"))
            .env("PATH", path)
            .env("FAKE_RUN", run_path)
            .env("FAKE_ARTIFACTS", artifacts_path)
            .env("FAKE_ZIP", self.temp.path().join("fixture-artifact.zip"))
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("GITHUB_REPOSITORY_ID", REPOSITORY_ID)
            .env("SOURCE_COMMIT", SOURCE_COMMIT)
            .env("STAGING_RUN_ID", RUN_ID)
            .env_remove("GH_TOKEN");
        if authenticated {
            command.env("GH_TOKEN", "fixture-token");
        }
        Ok(command.output()?)
    }
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/download-runtime-staging-artifact")
}

fn run_json(commit: &str) -> String {
    format!(
        r#"{{"id":42,"repository":{{"id":{REPOSITORY_ID}}},"head_branch":"main","head_sha":"{commit}","path":".github/workflows/runtime-candidate.yml","status":"completed","conclusion":"success","run_attempt":3}}"#
    )
}

fn artifacts_json(expired: bool, count: usize) -> String {
    let artifact = format!(
        r#"{{"id":7,"name":"runtime-staging-{RUN_ID}-3","expired":{expired}}}"#
    );
    format!(r#"{{"artifacts":[{}]}}"#, vec![artifact; count].join(","))
}

fn assert_failure(output: &Output, expected: &str) {
    assert!(!output.status.success(), "negative staging fixture unexpectedly passed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "missing {expected:?} in stderr: {stderr}"
    );
}
