use std::{fs, path::Path};

use crate::support::{FixtureCommand, fixture_path_text};

use super::fixture_matrix::{Change, Fixture, FixtureMatrix};

pub(super) struct BatchCase {
    name: &'static str,
    fixture: Fixture,
    state: &'static str,
    test_mode: bool,
}

impl BatchCase {
    pub(super) fn name(&self) -> &'static str { self.name }
}

pub(super) struct BatchResult {
    status: i32,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

impl BatchResult {
    pub(super) fn success(&self) -> bool { self.status == 0 }
}

impl FixtureMatrix {
    pub(super) fn batch_case(
        &self,
        name: &'static str,
        change: Change,
        state: &'static str,
        test_mode: bool,
    ) -> Result<BatchCase, Box<dyn std::error::Error>> {
        validate_field(name)?;
        Ok(BatchCase { name, fixture: self.case(change)?, state, test_mode })
    }

    pub(super) fn run_batch(
        &self,
        cases: &[BatchCase],
    ) -> Result<Vec<BatchResult>, Box<dyn std::error::Error>> {
        let root = self.temp.path().join("activation-verifier-batch");
        let manifest = root.join("cases.tsv");
        let results = root.join("results");
        fs::create_dir(&root)?;
        fs::write(&manifest, manifest_text(cases, &root)?)?;
        let mut path = vec![self.bin.clone()];
        path.extend(std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?));
        let script = codexy_runtime::paths::repository_root().join("scripts/verify-runtime-activation-branch");
        let mut command = FixtureCommand::new(script);
        command
            .args(["--batch"])
            .arg_path(&manifest)
            .arg_path(&results)
            .current_dir(self.temp.path())
            .env("CODEXY_TEST_ACTIVATE_RUNTIME", self.bin.join("activate"))
            .env_path_list("PATH", path)
            .env_remove("CODEXY_TEST_MODE")
            .env_remove("FAKE_PR_STATE_FILE");
        let output = command.output()?;
        assert!(output.status.success(), "batch entrypoint failed: {}", String::from_utf8_lossy(&output.stderr));
        self.verifier_starts.set(self.verifier_starts.get() + 1);
        self.batched_case_count.set(self.batched_case_count.get() + cases.len());
        cases.iter().map(|case| result(&results, case.name)).collect()
    }
}

fn manifest_text(cases: &[BatchCase], root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut records = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let state = root.join(format!("state-{index}"));
        fs::write(&state, case.state)?;
        let fields = [
            case.name.to_owned(),
            "activation".into(),
            "main".into(),
            case.fixture.activation_version.clone(),
            fixture_path_text(&case.fixture.receipt)?,
            fixture_path_text(&case.fixture.repo)?,
            fixture_path_text(&case.fixture.expected)?,
            fixture_path_text(&state)?,
            if case.test_mode { "1" } else { "0" }.into(),
        ];
        for field in &fields { validate_field(field)?; }
        records.push(fields.join("\t"));
    }
    Ok(format!("{}\n", records.join("\n")))
}

fn result(root: &Path, name: &str) -> Result<BatchResult, Box<dyn std::error::Error>> {
    let status = fs::read_to_string(root.join(format!("{name}.status")))?
        .trim()
        .parse()?;
    Ok(BatchResult {
        status,
        stdout: fs::read(root.join(format!("{name}.stdout")))?,
        stderr: fs::read(root.join(format!("{name}.stderr")))?,
    })
}

fn validate_field(value: &str) -> Result<(), Box<dyn std::error::Error>> {
    if value.is_empty() || value.contains(['\t', '\n', '\r']) {
        return Err("batch manifest field is unsafe".into());
    }
    Ok(())
}
