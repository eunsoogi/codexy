use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::{FixtureCommand, TestResult};
#[path = "review_control_direct_state.rs"]
mod direct_state;
#[path = "post_cap_review_graph.rs"]
mod graph;

pub(crate) fn validate_readiness(
    control: Value,
    issue_number: u64,
    head: &str,
) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let handoff = temporary.path().join("handoff.md");
    let state = temporary.path().join("state.json");
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let root_repair = control["post_cap_re_review"]["reason"].as_str()
        == Some("in_scope_contract_root_repair");
    let external_finding = control["post_cap_re_review"]["reason"].as_str()
        == Some("authenticated_external_finding_repair");
    let head = repository.resolve(head, root_repair, external_finding)?;
    let (control, _, current_base) = repository.prepare(
        &control,
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    fs::write(&handoff, "PASS on the exact current head.\n")?;
    fs::write(
        &state,
        serde_json::to_vec(&json!({
            "repository": "eunsoogi/codexy",
            "number": issue_number,
            "url": format!("https://github.com/eunsoogi/codexy/pull/{issue_number}"),
            "baseRefName": "main",
            "baseRefOid": current_base,
            "capture": {
                "provider": "github",
                "method": "graphql",
                "authenticated": true,
                "owningIssue": {
                    "repository": "eunsoogi/codexy",
                    "number": issue_number,
                    "url": format!("https://github.com/eunsoogi/codexy/issues/{issue_number}"),
                    "association": "owner-assignment"
                }
            },
            "state": "OPEN",
            "isDraft": true,
            "mergeStateStatus": "CLEAN",
            "headRefOid": head,
            "reviewProfile": control["profile"].clone(),
            "reviewControl": control
        }))?,
    )?;
    Ok(crate::support::validator_completion_handoff_files(
        &handoff, &state,
    )?)
}
pub(crate) fn build_pr_state(
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let (control, previous_base, current_base) =
        repository.prepare(control, previous_base, current_base)?;
    let output = temporary.path().join("pr-state.json");
    let (current, control_path, previous) = write_review_inputs(
        temporary.path(),
        &control,
        &previous_base,
        &current_base,
    )?;
    let result = invoke_build(
        &repository.path,
        &current,
        &control_path,
        &previous,
        &output,
    )?;
    assert!(
        result.status.success(),
        "build-pr-state must accept the typed post-cap state: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(serde_json::from_slice(&fs::read(output)?)?)
}
pub(crate) fn run_build(
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let (control, previous_base, current_base) =
        repository.prepare(control, previous_base, current_base)?;
    let output = temporary.path().join("pr-state.json");
    let (current, control_path, previous) = write_review_inputs(
        temporary.path(),
        &control,
        &previous_base,
        &current_base,
    )?;
    Ok(invoke_build(
        &repository.path,
        &current,
        &control_path,
        &previous,
        &output,
    )?)
}
pub(crate) fn produce(
    control: &Value,
    source: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let mut source_control = control.clone();
    let retained_capture = source_control["post_cap_re_review"]["qualifying_change"]
        ["external_finding"]["capture"]
        .clone();
    source_control["post_cap_re_review"]["qualifying_change"]["external_finding"] =
        source.clone();
    if retained_capture.is_object() {
        source_control["post_cap_re_review"]["qualifying_change"]["external_finding"]["capture"] =
            retained_capture;
    }
    let (mut control, previous_base, current_base) =
        repository.prepare(&source_control, previous_base, current_base)?;
    let source = control["post_cap_re_review"]["qualifying_change"]["external_finding"].clone();
    let change = control["post_cap_re_review"]["qualifying_change"]
        .as_object_mut()
        .ok_or("qualifying change")?;
    change.remove("external_finding");
    change.remove("finding_ids");
    let current_head = control["reviewed_head"].as_str().ok_or("current head")?;
    let previous_head = control["terminal_review_history"][1]["reviewed_head"]
        .as_str()
        .ok_or("previous head")?;
    let current = direct_state::pr_snapshot(
        control["issue_number"].as_u64().ok_or("issue number")?,
        &current_base,
        current_head,
        None,
    );
    let previous = direct_state::pr_snapshot(
        control["issue_number"].as_u64().ok_or("issue number")?,
        &previous_base,
        previous_head,
        Some(direct_state::post_cap_prior(&control)),
    );
    let input = temporary.path().join("producer-input.json");
    let output = temporary.path().join("producer-output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_external_finding_capture": source["capture"].clone(),
            "authenticated_external_finding": source,
            "current_pr_state": current,
            "previous_pr_state": previous
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .args(["--repository-root"])
        .arg(&repository.path)
        .status()?;
    if !result.success() { return Err(std::io::Error::other("producer rejected").into()); }
    Ok(serde_json::from_slice(&fs::read(output)?)?)
}
fn write_review_inputs(
    root: &std::path::Path,
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<(
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
)> {
    let issue_number = control["issue_number"].as_u64().ok_or("issue number")?;
    let previous = direct_state::post_cap_prior(control);
    let previous_head = previous["reviewed_head"]
        .as_str()
        .ok_or("prior head")?
        .to_owned();
    let current_head = control["reviewed_head"].as_str().ok_or("current head")?;
    let current = root.join("current-pr-state.json");
    let control_path = root.join("review-control.json");
    let previous_path = root.join("previous-pr-state.json");
    fs::write(
        &current,
        serde_json::to_vec(&direct_state::pr_snapshot(
            issue_number,
            current_base,
            current_head,
            None,
        ))?,
    )?;
    fs::write(&control_path, serde_json::to_vec(control)?)?;
    fs::write(
        &previous_path,
        serde_json::to_vec(&direct_state::pr_snapshot(
            issue_number,
            previous_base,
            &previous_head,
            Some(previous),
        ))?,
    )?;
    Ok((current, control_path, previous_path))
}
fn invoke_build(
    repository: &Path,
    current: &std::path::Path,
    control: &std::path::Path,
    previous: &std::path::Path,
    output: &std::path::Path,
) -> TestResult<std::process::Output> {
    let mut command = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    command
        .arg("--repository-root")
        .arg_path(repository)
        .arg("--base-pr-state-file")
        .arg_path(current)
        .arg("--review-control-state-file")
        .arg_path(control)
        .arg("--previous-pr-state-file")
        .arg_path(previous)
        .arg("--output")
        .arg_path(output)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        );
    Ok(command.output()?)
}
