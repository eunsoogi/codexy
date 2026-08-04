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
fn downloads_authenticated_staging_with_space_safe_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(&run_json(SOURCE_COMMIT), &artifacts_json(false, 1), true, SOURCE_COMMIT)?;
    assert!(
        output.status.success(),
        "authenticated staging download failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture.root.join("staging/runtime-staging-receipt.json").is_file());
    assert!(fixture.root.join("staging/runtime-staging-run.json").is_file());
    assert_eq!(
        fixture.root.file_name().and_then(|path| path.to_str()),
        Some("staging fixture with spaces")
    );
    Ok(())
}

#[test]
fn rejects_mismatched_staging_identity() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(
        &run_json(SOURCE_COMMIT),
        &artifacts_json(false, 1),
        true,
        "ffffffffffffffffffffffffffffffffffffffff",
    )?;
    assert_failure(&output, "staging receipt source commit mismatch");
    Ok(())
}

#[test]
fn accepts_an_ancestor_input_when_the_dispatch_run_is_on_main()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(
        &run_json_at_head("fedcba9876543210fedcba9876543210fedcba98"),
        &artifacts_json(false, 1),
        true,
        SOURCE_COMMIT,
    )?;
    assert!(
        output.status.success(),
        "protected-main staging run rejected its distinct input: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn rejects_expired_staging_artifact() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(&run_json(SOURCE_COMMIT), &artifacts_json(true, 1), true, SOURCE_COMMIT)?;
    assert_failure(&output, "staging artifact expired");
    Ok(())
}

#[test]
fn rejects_duplicate_staging_artifact_identity() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(&run_json(SOURCE_COMMIT), &artifacts_json(false, 2), true, SOURCE_COMMIT)?;
    assert_failure(&output, "staging artifact identity is not unique");
    Ok(())
}

#[test]
fn rejects_unauthenticated_staging_download() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let output = fixture.run(&run_json(SOURCE_COMMIT), &artifacts_json(false, 1), false, SOURCE_COMMIT)?;
    assert_failure(&output, "authenticated GitHub token is required");
    Ok(())
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    gh: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("staging fixture with spaces");
        fs::create_dir(&root)?;
        let gh = root.join("gh");
        fs::write(
            &gh,
            "#!/bin/sh\ncase \"$*\" in\n  *'/actions/artifacts/'*'/zip') cat \"$FAKE_ZIP\" ;;\n  *'/artifacts') cat \"$FAKE_ARTIFACTS\" ;;\n  *'/actions/runs/'*) cat \"$FAKE_RUN\" ;;\n  *) exit 91 ;;\nesac\n",
        )?;
        support::make_executable(&gh)?;
        Ok(Self { _temp: temp, root, gh })
    }

    fn run(
        &self,
        run: &str,
        artifacts: &str,
        authenticated: bool,
        receipt_source: &str,
    ) -> Result<Output, Box<dyn std::error::Error>> {
        let run_path = self.root.join("run.json");
        let artifacts_path = self.root.join("artifacts.json");
        fs::write(&run_path, run)?;
        fs::write(&artifacts_path, artifacts)?;
        let receipt = self.root.join("receipt.json");
        fs::write(
            &receipt,
            format!(
                r#"{{"candidate":{{"source":{{"commit":"{receipt_source}"}},"artifact":{{"stagingRunId":42,"stagingRunAttempt":3}}}}}}"#
            ),
        )?;
        let archive = self.root.join("fixture-artifact.zip");
        let status = Command::new("python3")
            .args(["-c", "import pathlib,sys,zipfile; z=zipfile.ZipFile(sys.argv[1],'w'); z.writestr('runtime-staging-receipt.json', pathlib.Path(sys.argv[2]).read_text()); z.close()"])
            .arg(&archive)
            .arg(&receipt)
            .status()?;
        assert!(status.success(), "failed to create staging zip fixture");
        let host_path = std::env::var_os("PATH").ok_or("host PATH")?;
        let mut path_entries = vec![self.gh.parent().ok_or("fake gh parent")?.to_path_buf()];
        path_entries.extend(std::env::split_paths(&host_path));
        let mut command = Command::new(script());
        command
            .arg_path(self.root.join("staging"))
            .env_path_list("PATH", path_entries)
            .env_path("FAKE_RUN", run_path)
            .env_path("FAKE_ARTIFACTS", artifacts_path)
            .env_path("FAKE_ZIP", archive)
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
    run_json_at_head(commit)
}

fn run_json_at_head(head_commit: &str) -> String {
    format!(
        r#"{{"id":42,"repository":{{"id":{REPOSITORY_ID}}},"head_branch":"main","head_sha":"{head_commit}","path":".github/workflows/runtime-candidate.yml","status":"completed","conclusion":"success","run_attempt":3}}"#
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
