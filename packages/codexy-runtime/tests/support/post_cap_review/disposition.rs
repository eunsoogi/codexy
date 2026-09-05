use std::fs;

use serde_json::{Value, json};

use crate::support::{FixtureCommand, TestResult, make_executable};

use super::{
    direct_state, disposition_fixture, graph, invoke_build, write_review_inputs,
};

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
    let disposition = control["post_cap_re_review"]["reason"].as_str()
        == Some("authenticated_finding_disposition");
    let head = repository.resolve(head, root_repair, external_finding, disposition)?;
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

pub(crate) fn produce_disposition(mut control: Value) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let change = control["post_cap_re_review"]["qualifying_change"]
        .as_object_mut()
        .ok_or("disposition qualifying change")?;
    change.remove("finding_ids");
    change.remove("finding_disposition");
    let (control, previous_base, current_base) = repository.prepare(
        &control,
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    let issue = control["issue_number"].as_u64().ok_or("disposition issue")?;
    let current_head = control["reviewed_head"]
        .as_str()
        .ok_or("disposition current head")?;
    let previous = direct_state::post_cap_prior(&control);
    let previous_head = previous["reviewed_head"]
        .as_str()
        .ok_or("disposition previous head")?
        .to_owned();
    let current = direct_state::pr_snapshot(issue, &current_base, current_head, None);
    let previous = direct_state::pr_snapshot(issue, &previous_base, &previous_head, Some(previous));
    let input = temporary.path().join("producer-input.json");
    let output = temporary.path().join("producer-output.json");
    let ci_response = temporary.path().join("ci-response.json");
    let maintainer_response = temporary.path().join("maintainer-response.json");
    fs::write(
        &ci_response,
        serde_json::to_vec(&disposition_fixture::ci_response(
            issue,
            &current_base,
            current_head,
        ))?,
    )?;
    fs::write(
        &maintainer_response,
        serde_json::to_vec(&disposition_fixture::maintainer_response(
            issue,
            issue,
            &current_base,
            current_head,
        ))?,
    )?;
    let bin = temporary.path().join("bin");
    fs::create_dir(&bin)?;
    let gh = bin.join("gh");
    fs::write(
        &gh,
        "#!/bin/sh\nif [ \"$1\" = \"pr\" ]; then cat \"$CODEXY_TEST_CI_RESPONSE\"; else cat \"$CODEXY_TEST_MAINTAINER_RESPONSE\"; fi\n",
    )?;
    make_executable(&gh)?;
    let mut paths = vec![bin];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_finding_disposition_locator": {
                "repository": "eunsoogi/codexy",
                "owningIssue": issue,
                "pullRequest": issue,
                "maintainerComment": 5554573060u64
            },
            "current_pr_state": current,
            "previous_pr_state": previous
        }))?,
    )?;
    let mut command = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    command
        .args(["--produce-review-control", "--input"])
        .arg_path(&input)
        .args(["--output"])
        .arg_path(&output)
        .args(["--repository-root"])
        .arg_path(&repository.path)
        .env_path_list("PATH", paths)
        .env_path("CODEXY_TEST_CI_RESPONSE", ci_response)
        .env_path("CODEXY_TEST_MAINTAINER_RESPONSE", maintainer_response);
    let result = command.output()?;
    assert!(
        result.status.success(),
        "finding disposition producer must accept live sources: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(serde_json::from_slice(&fs::read(output)?)?)
}

pub(crate) fn run_build_with_disposition_maintainer<F>(
    control: &Value,
    previous_base: &str,
    current_base: &str,
    make_maintainer_response: F,
) -> TestResult<std::process::Output>
where
    F: FnOnce(u64, &str, &str) -> Value,
{
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
    let state: Value = serde_json::from_slice(&fs::read(&current)?)?;
    let pull = state["number"].as_u64().ok_or("disposition pull")?;
    let head = state["headRefOid"].as_str().ok_or("disposition head")?;
    let maintainer_response = make_maintainer_response(pull, &current_base, head);
    let ci_response = disposition_fixture::ci_response(pull, &current_base, head);
    Ok(invoke_build(
        &repository.path,
        &current,
        &control_path,
        &previous,
        &output,
        &control,
        Some((ci_response, maintainer_response)),
    )?)
}
