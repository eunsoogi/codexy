use std::path::Path;
use std::process::Output;

use crate::support;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "validator_sentinel_reviewer_gate/approval_boundaries.rs"]
mod approval_boundaries;
#[path = "validator_sentinel_reviewer_gate/baseline_contract.rs"]
mod baseline_contract;
#[path = "validator_sentinel_reviewer_gate/evidence_regressions.rs"]
mod evidence_regressions;

fn validate_sentinel_replacement(needle: &str, replacement: &str) -> TestResult<Output> {
    validate_sentinel_edit(|sentinel| sentinel.replace(needle, replacement))
}

fn validate_sentinel_edit(edit: impl FnOnce(String) -> String) -> TestResult<Output> {
    let fixture = support::roles_fixture()?;
    let sentinel_path = fixture.root().join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    std::fs::write(&sentinel_path, edit(sentinel))?;
    support::validator_in_process(fixture.root(), "--check-roles")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
