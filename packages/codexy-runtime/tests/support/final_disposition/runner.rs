use std::fs;

use serde_json::{Value, json};

use super::{builder, fixture, graph};
use crate::support::{FixtureCommand, TestResult};

pub(crate) fn produce(control: &Value, previous_control: &Value) -> TestResult<Value> {
    let issue = control["issue_number"].as_u64().ok_or("final disposition issue")?;
    produce_for(control, previous_control, issue)
}

pub(crate) fn produce_for(
    control: &Value,
    previous_control: &Value,
    pull_request: u64,
) -> TestResult<Value> {
    let result = run_producer(control, previous_control, pull_request, true, |_| Ok(()))?;
    if !result.status.success() {
        return Err(format!(
            "final disposition producer failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&result.stdout)?)
}

pub(crate) fn canonical_third_predecessor(
    control: &Value,
    current: &Value,
    previous: &Value,
) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("third-predecessor-input.json");
    let output = temporary.path().join("third-predecessor-output.json");
    let predecessor = super::third_block_predecessor(control);
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
        .arg(codexy_runtime::paths::repository_root())
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

pub(crate) fn produce_with_states(
    control: &Value,
    current: &Value,
    previous: &Value,
    pull_request: u64,
) -> TestResult<Value> {
    let result = run_producer_with_states(control, current, previous, pull_request)?;
    if !result.status.success() {
        return Err(format!(
            "final disposition producer with native history failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&result.stdout)?)
}

pub(crate) fn produce_without_locator(
    control: &Value,
    previous_control: &Value,
) -> TestResult<std::process::Output> {
    let issue = control["issue_number"].as_u64().ok_or("final disposition issue")?;
    run_producer(control, previous_control, issue, false, |_| Ok(()))
}

pub(crate) fn produce_with_fixture_mutation<F>(
    control: &Value,
    previous_control: &Value,
    pull_request: u64,
    mutate: F,
) -> TestResult<std::process::Output>
where
    F: FnOnce(&fixture::GhFixture) -> TestResult<()>,
{
    run_producer(control, previous_control, pull_request, true, mutate)
}

fn run_producer<F>(
    control: &Value,
    previous_control: &Value,
    pull_request: u64,
    include_locator: bool,
    mutate: F,
) -> TestResult<std::process::Output>
where
    F: FnOnce(&fixture::GhFixture) -> TestResult<()>,
{
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let issue = control["issue_number"].as_u64().ok_or("final disposition issue")?;
    let base_seed = control["final_disposition"]["base_oid"]
        .as_str()
        .ok_or("final disposition base")?;
    let (control, previous_base, current_base) =
        repository.prepare(control, base_seed, base_seed)?;
    let (previous_control, _, _) = repository.prepare(previous_control, base_seed, base_seed)?;
    let current_head = control["final_disposition"]["head_oid"]
        .as_str()
        .ok_or("final disposition current head")?
        .to_owned();
    let previous_head = previous_control["reviewed_head"]
        .as_str()
        .ok_or("final disposition previous head")?
        .to_owned();
    let current = builder::snapshot(pull_request, issue, &current_base, &current_head, None);
    let previous = builder::snapshot(
        pull_request,
        issue,
        &previous_base,
        &previous_head,
        Some(previous_control),
    );
    let source_head = control["final_disposition"]["source_head"]
        .as_str()
        .ok_or("final disposition source head")?;
    let finding_id = control["final_disposition"]["addressed_finding_ids"][0]
        .as_str()
        .ok_or("final disposition finding id")?;
    let fixture = fixture::write(
        temporary.path(),
        issue,
        pull_request,
        &current_base,
        source_head,
        &current_head,
        finding_id,
    )?;
    mutate(&fixture)?;
    let input = temporary.path().join("producer-input.json");
    let output = temporary.path().join("producer-output.json");
    let mut request = json!({
        "control_state": control,
        "current_pr_state": current,
        "previous_pr_state": previous
    });
    if include_locator {
        request["authenticated_final_disposition_locator"] =
            fixture::locator(issue, pull_request);
    }
    fs::write(
        &input,
        serde_json::to_vec(&request)?,
    )?;
    let mut command = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    fixture::configure(&mut command, &fixture);
    let mut result = command
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output", output.to_str().ok_or("producer output path")?])
        .args(["--repository-root", repository.path.to_str().ok_or("repository path")?])
        .output()?;
    if result.status.success() {
        result.stdout = fs::read(output)?;
    }
    Ok(result)
}

fn run_producer_with_states(
    control: &Value,
    current: &Value,
    previous: &Value,
    pull_request: u64,
) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let issue = control["issue_number"].as_u64().ok_or("final disposition issue")?;
    let base = current["baseRefOid"].as_str().ok_or("current base")?;
    let head = current["headRefOid"].as_str().ok_or("current head")?;
    let source_head = control["final_disposition"]["source_head"]
        .as_str()
        .ok_or("final disposition source head")?;
    let finding_id = control["final_disposition"]["addressed_finding_ids"][0]
        .as_str()
        .ok_or("final disposition finding id")?;
    let fixture = fixture::write(
        temporary.path(),
        issue,
        pull_request,
        base,
        source_head,
        head,
        finding_id,
    )?;
    let predecessor = canonical_third_predecessor(control, current, previous)?;
    let mut previous_state = previous.clone();
    previous_state["reviewControl"] = predecessor;
    let input = temporary.path().join("producer-input.json");
    let output = temporary.path().join("producer-output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_final_disposition_locator": fixture::locator(issue, pull_request),
            "current_pr_state": current,
            "previous_pr_state": previous_state
        }))?,
    )?;
    let mut command = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    fixture::configure(&mut command, &fixture);
    let mut result = command
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output", output.to_str().ok_or("producer output path")?])
        .args(["--repository-root"])
        .arg(codexy_runtime::paths::repository_root())
        .output()?;
    if result.status.success() {
        result.stdout = fs::read(output)?;
    }
    Ok(result)
}
