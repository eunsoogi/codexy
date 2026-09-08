use std::fs;

use serde_json::{Value, json};

use super::{graph, native_919};
use crate::support::{FixtureCommand, TestResult};

pub(crate) fn canonical_third_predecessor(
    control: &Value,
    current: &Value,
    previous: &Value,
) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let repair_head = repository.commit_finding_repair(native_919::REPAIR_PATH)?;
    canonical_third_predecessor_in_repository(
        &repository,
        &repair_head,
        control,
        current,
        previous,
    )
}

pub(crate) fn canonical_third_predecessor_in_repository(
    repository: &graph::SyntheticRepository,
    repair_head: &str,
    control: &Value,
    current: &Value,
    previous: &Value,
) -> TestResult<Value> {
    let input = repository.path.join("third-predecessor-input.json");
    let output = repository.path.join("third-predecessor-output.json");
    let predecessor = native_919::localize_for_repository(
        repository,
        &super::third_block_predecessor(control),
        repair_head,
    )?;
    let current = native_919::localize_for_repository(repository, current, repair_head)?;
    let previous = native_919::localize_for_repository(repository, previous, repair_head)?;
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": predecessor,
            "current_pr_state": current,
            "previous_pr_state": previous
        }))?,
    )?;
    let result = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .args(["--repository-root"])
        .arg(&repository.path)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "native-history third predecessor failed: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&fs::read(output)?)?)
}
