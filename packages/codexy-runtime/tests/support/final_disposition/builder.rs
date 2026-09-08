use std::fs;

use serde_json::{Value, json};

use super::{direct_state, fixture, graph, native_919};
use crate::support::{FixtureCommand, TestResult};

pub(crate) fn build_pr_state(control: &Value, previous_control: &Value) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let issue = control["issue_number"].as_u64().ok_or("final disposition issue")?;
    let base_seed = control["final_disposition"]["base_oid"]
        .as_str()
        .ok_or("final disposition base")?;
    let (control, _, current_base) = repository.prepare(control, base_seed, base_seed)?;
    let (previous_control, previous_base, _) =
        repository.prepare(previous_control, base_seed, base_seed)?;
    let current_head = control["final_disposition"]["head_oid"]
        .as_str()
        .ok_or("final disposition current head")?
        .to_owned();
    let previous_head = previous_control["reviewed_head"]
        .as_str()
        .ok_or("final disposition previous head")?
        .to_owned();
    let current_path = temporary.path().join("current-pr-state.json");
    let input_path = temporary.path().join("producer-input.json");
    let control_path = temporary.path().join("review-control.json");
    let previous_path = temporary.path().join("previous-pr-state.json");
    let output_path = temporary.path().join("pr-state.json");
    let source_head = control["final_disposition"]["source_head"]
        .as_str()
        .ok_or("final disposition source head")?;
    let finding_id = control["final_disposition"]["addressed_finding_ids"][0]
        .as_str()
        .ok_or("final disposition finding id")?;
    let fixture = fixture::write(
        temporary.path(),
        issue,
        issue,
        &current_base,
        source_head,
        &current_head,
        finding_id,
    )?;
    fs::write(
        &input_path,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_final_disposition_locator": fixture::locator(issue, issue),
            "current_pr_state": snapshot(issue, issue, &current_base, &current_head, None),
            "previous_pr_state": snapshot(
                issue,
                issue,
                &previous_base,
                &previous_head,
                Some(previous_control.clone()),
            )
        }))?,
    )?;
    fs::write(
        &current_path,
        serde_json::to_vec(&snapshot(issue, issue, &current_base, &current_head, None))?,
    )?;
    fs::write(
        &previous_path,
        serde_json::to_vec(&snapshot(
            issue,
            issue,
            &previous_base,
            &previous_head,
            Some(previous_control),
        ))?,
    )?;
    let mut producer = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    fixture::configure(&mut producer, &fixture);
    let result = producer
        .args(["--produce-review-control", "--input"])
        .arg_path(&input_path)
        .args(["--output"])
        .arg_path(&control_path)
        .args(["--repository-root"])
        .arg_path(&repository.path)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "final disposition producer before build failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    let mut build = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    fixture::configure(&mut build, &fixture);
    let result = build
        .args(["--repository-root"])
        .arg_path(&repository.path)
        .args(["--base-pr-state-file"])
        .arg_path(&current_path)
        .args(["--review-control-state-file"])
        .arg_path(&control_path)
        .args(["--previous-pr-state-file"])
        .arg_path(&previous_path)
        .args(["--output"])
        .arg_path(&output_path)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        )
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "final disposition build-pr-state failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&fs::read(output_path)?)?)
}

pub(crate) fn build_pr_state_with_states(
    control: &Value,
    current: &Value,
    previous: &Value,
    pull_request: u64,
) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let repair_head = repository.commit_finding_repair(native_919::REPAIR_PATH)?;
    let control = native_919::localize_for_repository(&repository, control, &repair_head)?;
    let current = native_919::localize_for_repository(&repository, current, &repair_head)?;
    let previous = native_919::localize_for_repository(&repository, previous, &repair_head)?;
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
    let input_path = temporary.path().join("producer-input.json");
    let control_path = temporary.path().join("review-control.json");
    let current_path = temporary.path().join("current-pr-state.json");
    let previous_path = temporary.path().join("previous-pr-state.json");
    let output_path = temporary.path().join("pr-state.json");
    let predecessor = super::runner::canonical_third_predecessor_in_repository(
        &repository,
        &repair_head,
        &control,
        &current,
        &previous,
    )?;
    let mut previous_state = previous.clone();
    previous_state["reviewControl"] = predecessor;
    fs::write(
        &input_path,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_final_disposition_locator": fixture::locator(issue, pull_request),
            "current_pr_state": current,
            "previous_pr_state": previous_state
        }))?,
    )?;
    fs::write(&current_path, serde_json::to_vec(&current)?)?;
    fs::write(&previous_path, serde_json::to_vec(&previous_state)?)?;
    let mut producer = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    fixture::configure(&mut producer, &fixture);
    let result = producer
        .args(["--produce-review-control", "--input"])
        .arg(&input_path)
        .args(["--output"])
        .arg(&control_path)
        .args(["--repository-root"])
        .arg(&repository.path)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "native-history producer before build failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    let mut build = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    fixture::configure(&mut build, &fixture);
    let result = build
        .args(["--repository-root"])
        .arg(&repository.path)
        .args(["--base-pr-state-file"])
        .arg(&current_path)
        .args(["--review-control-state-file"])
        .arg(&control_path)
        .args(["--previous-pr-state-file"])
        .arg(&previous_path)
        .args(["--output"])
        .arg(&output_path)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        )
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "native-history build-pr-state failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&fs::read(output_path)?)?)
}

pub(crate) fn snapshot(
    pull_request: u64,
    issue: u64,
    base: &str,
    head: &str,
    control: Option<Value>,
) -> Value {
    let mut snapshot = direct_state::pr_snapshot(pull_request, base, head, control);
    snapshot["capture"]["owningIssue"]["number"] = json!(issue);
    snapshot["capture"]["owningIssue"]["url"] =
        json!(format!("https://github.com/eunsoogi/codexy/issues/{issue}"));
    snapshot["state"] = json!("OPEN");
    snapshot["isDraft"] = json!(false);
    snapshot["mergeStateStatus"] = json!("CLEAN");
    snapshot["reviewThreads"] = json!({"pageInfo": {"hasNextPage": false}, "nodes": []});
    snapshot
}
