use std::{
    cell::Cell,
    fs,
    path::PathBuf,
};

use crate::support::{self, fixture_path_text, FixtureCommand as Command};

use super::staging_zip_fixture;

const REPOSITORY_ID: &str = "1269350143";
const RUN_ID: &str = "42";
const SOURCE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn batch_fixture_preserves_each_artifact_download_outcome()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let outcomes = fixture.run_batch(&[
        case("authenticated", run_json(SOURCE_COMMIT), artifacts_json(false, 1), true, SOURCE_COMMIT, 42, 3),
        case("ancestor-input", run_json_at_head("fedcba9876543210fedcba9876543210fedcba98"), artifacts_json(false, 1), true, SOURCE_COMMIT, 42, 3),
        case("source-mismatch", run_json(SOURCE_COMMIT), artifacts_json(false, 1), true, "ffffffffffffffffffffffffffffffffffffffff", 42, 3),
        case("malformed-head", run_json_at_head("not-a-commit"), artifacts_json(false, 1), true, SOURCE_COMMIT, 42, 3),
        case("run-mismatch", run_json(SOURCE_COMMIT), artifacts_json(false, 1), true, SOURCE_COMMIT, 99, 3),
        case("attempt-mismatch", run_json(SOURCE_COMMIT), artifacts_json(false, 1), true, SOURCE_COMMIT, 42, 4),
        case("expired", run_json(SOURCE_COMMIT), artifacts_json(true, 1), true, SOURCE_COMMIT, 42, 3),
        case("duplicate", run_json(SOURCE_COMMIT), artifacts_json(false, 2), true, SOURCE_COMMIT, 42, 3),
        case("unauthenticated", run_json(SOURCE_COMMIT), artifacts_json(false, 1), false, SOURCE_COMMIT, 42, 3),
    ])?;
    assert_eq!(fixture.batch_starts.get(), 1, "one outer fixture process must run all cases");
    assert_eq!(outcomes.len(), 9);
    for index in [0, 1] {
        assert!(outcomes[index].success(), "{} failed: {}", outcomes[index].name, String::from_utf8_lossy(&outcomes[index].stderr));
        assert!(outcomes[index].root.join("staging/runtime-staging-receipt.json").is_file());
        assert!(outcomes[index].root.join("staging/runtime-staging-run.json").is_file());
        assert_eq!(outcomes[index].root.file_name().and_then(|path| path.to_str()), Some("staging fixture with spaces"));
    }
    for (index, expected) in [
        (2, "staging receipt source commit mismatch"),
        (3, "staging run head commit is invalid"),
        (4, "staging receipt run identity mismatch"),
        (5, "staging receipt run attempt mismatch"),
        (6, "staging artifact expired"),
        (7, "staging artifact identity is not unique"),
        (8, "authenticated GitHub token is required"),
    ] { assert_failure(&outcomes[index], expected); }
    Ok(())
}

struct BatchCase {
    name: &'static str,
    run: String,
    artifacts: String,
    authenticated: bool,
    receipt_source: &'static str,
    receipt_run_id: u64,
    receipt_run_attempt: u64,
}

fn case(name: &'static str, run: String, artifacts: String, authenticated: bool, receipt_source: &'static str, receipt_run_id: u64, receipt_run_attempt: u64) -> BatchCase {
    BatchCase { name, run, artifacts, authenticated, receipt_source, receipt_run_id, receipt_run_attempt }
}

struct BatchResult { name: &'static str, root: PathBuf, status: i32, stderr: Vec<u8> }

impl BatchResult { fn success(&self) -> bool { self.status == 0 } }

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    gh: PathBuf,
    batch_starts: Cell<usize>,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("artifact download batch");
        fs::create_dir(&root)?;
        let gh = root.join("gh");
        fs::write(&gh, "#!/bin/sh\ncase \"$*\" in\n  *'/actions/artifacts/'*'/zip') cat \"$FAKE_ZIP\" ;;\n  *'/artifacts') cat \"$FAKE_ARTIFACTS\" ;;\n  *'/actions/runs/'*) cat \"$FAKE_RUN\" ;;\n  *) exit 91 ;;\nesac\n")?;
        support::make_executable(&gh)?;
        Ok(Self { _temp: temp, root, gh, batch_starts: Cell::new(0) })
    }

    fn run_batch(&self, cases: &[BatchCase]) -> Result<Vec<BatchResult>, Box<dyn std::error::Error>> {
        let manifest = self.root.join("cases.tsv");
        let results = self.root.join("results");
        let mut rows = Vec::with_capacity(cases.len());
        let mut roots = Vec::with_capacity(cases.len());
        for (index, case) in cases.iter().enumerate() {
            let root = self.root.join(format!("case-{index}/staging fixture with spaces"));
            fs::create_dir_all(&root)?;
            let run = root.join("run.json");
            let artifacts = root.join("artifacts.json");
            let receipt = root.join("receipt.json");
            let archive = root.join("fixture-artifact.zip");
            fs::write(&run, &case.run)?;
            fs::write(&artifacts, &case.artifacts)?;
            fs::write(&receipt, format!(r#"{{"candidate":{{"source":{{"commit":"{}"}},"artifact":{{"stagingRunId":{},"stagingRunAttempt":{}}}}}}}"#, case.receipt_source, case.receipt_run_id, case.receipt_run_attempt))?;
            staging_zip_fixture::write_receipt_archive(&archive, &fs::read(&receipt)?)?;
            rows.push([case.name.to_string(), fixture_path_text(root.join("staging"))?, fixture_path_text(run)?, fixture_path_text(artifacts)?, fixture_path_text(archive)?, if case.authenticated { "1" } else { "0" }.into()].join("\t"));
            roots.push(root);
        }
        fs::write(&manifest, format!("{}\n", rows.join("\n")))?;
        let runner = self.root.join("run-batch.sh");
        fs::write(&runner, BATCH_RUNNER)?;
        support::make_executable(&runner)?;
        let mut path = vec![self.gh.parent().ok_or("fake gh parent")?.to_path_buf()];
        path.extend(std::env::split_paths(&std::env::var_os("PATH").ok_or("host PATH")?));
        let mut command = Command::new(&runner);
        command.arg_path(&manifest).arg_path(&results).arg_path(script()).env_path_list("PATH", path).env("GITHUB_REPOSITORY", "eunsoogi/codexy").env("GITHUB_REPOSITORY_ID", REPOSITORY_ID).env("SOURCE_COMMIT", SOURCE_COMMIT).env("STAGING_RUN_ID", RUN_ID).env_remove("GH_TOKEN");
        let output = command.output()?;
        assert!(output.status.success(), "batch runner failed: {}", String::from_utf8_lossy(&output.stderr));
        self.batch_starts.set(self.batch_starts.get() + 1);
        cases.iter().zip(roots).map(|(case, root)| Ok(BatchResult { name: case.name, root, status: fs::read_to_string(results.join(format!("{}.status", case.name)))?.trim().parse()?, stderr: fs::read(results.join(format!("{}.stderr", case.name)))? })).collect()
    }
}

const BATCH_RUNNER: &str = r##"#!/bin/sh
set -u
manifest=$1
results=$2
downloader=$3
mkdir "$results"
tab="$(printf '\t')"
while IFS="$tab" read -r name output run artifacts archive authenticated; do
  if test "$authenticated" = 1; then
    GH_TOKEN=fixture-token FAKE_RUN="$run" FAKE_ARTIFACTS="$artifacts" FAKE_ZIP="$archive" "$downloader" "$output" > "$results/$name.stdout" 2> "$results/$name.stderr"
  else
    FAKE_RUN="$run" FAKE_ARTIFACTS="$artifacts" FAKE_ZIP="$archive" "$downloader" "$output" > "$results/$name.stdout" 2> "$results/$name.stderr"
  fi
  printf '%s\n' "$?" > "$results/$name.status"
done < "$manifest"
"##;

fn script() -> PathBuf { codexy_runtime::paths::repository_root().join("scripts/download-runtime-staging-artifact") }

fn run_json(commit: &str) -> String { run_json_at_head(commit) }

fn run_json_at_head(head_commit: &str) -> String {
    format!(r#"{{"id":42,"repository":{{"id":{REPOSITORY_ID}}},"head_branch":"main","head_sha":"{head_commit}","path":".github/workflows/runtime-candidate.yml","status":"completed","conclusion":"success","run_attempt":3}}"#)
}

fn artifacts_json(expired: bool, count: usize) -> String {
    let artifact = format!(r#"{{"id":7,"name":"runtime-staging-{RUN_ID}-3","expired":{expired}}}"#);
    format!(r#"{{"artifacts":[{}]}}"#, vec![artifact; count].join(","))
}

fn assert_failure(output: &BatchResult, expected: &str) {
    assert!(!output.success(), "{} unexpectedly passed", output.name);
    assert!(String::from_utf8_lossy(&output.stderr).contains(expected), "{} missing {expected:?}: {}", output.name, String::from_utf8_lossy(&output.stderr));
}
