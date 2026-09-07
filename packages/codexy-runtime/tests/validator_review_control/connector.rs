use std::{fs, process::Command};

use crate::support::TestResult;
use serde_json::{Value, json};

use super::{BASE_OID, HEAD_OID, direct_state};

#[test]
fn review_control_producer_validates_connector_snapshot_provenance() -> TestResult {
    let current = direct_state::connector_pr_snapshot_without_derived(
        17, 11, BASE_OID, HEAD_OID, None,
    );
    let previous = direct_state::connector_pr_snapshot_without_derived(
        17,
        11,
        BASE_OID,
        BASE_OID,
        Some(direct_state::strict_genesis(11)),
    );
    let (result, produced) = run_producer(
        &current,
        &direct_state::strict_control(11, HEAD_OID),
        &previous,
    )?;
    assert!(
        result.status.success(),
        "connector source must reach the canonical producer: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(produced.expect("producer output")["issue_number"], 11);
    Ok(())
}

#[test]
fn review_control_producer_rejects_connector_derived_field_contradiction() -> TestResult {
    let mut current = direct_state::connector_pr_snapshot_without_derived(
        17, 11, BASE_OID, HEAD_OID, None,
    );
    current["number"] = json!(18);
    let previous = direct_state::connector_pr_snapshot_without_derived(
        17,
        11,
        BASE_OID,
        BASE_OID,
        Some(direct_state::strict_genesis(11)),
    );
    let (result, produced) = run_producer(
        &current,
        &direct_state::strict_control(11, HEAD_OID),
        &previous,
    )?;
    assert!(
        !result.status.success(),
        "producer must reject a contradictory derived connector field"
    );
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("connector arguments"),
        "contradiction diagnostic must name connector source: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(produced.is_none());
    Ok(())
}

fn run_producer(
    current: &Value,
    control: &Value,
    previous: &Value,
) -> TestResult<(std::process::Output, Option<Value>)> {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "current_pr_state": current,
            "previous_pr_state": previous
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    let produced = result
        .status
        .success()
        .then(|| fs::read(&output))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    Ok((result, produced))
}
