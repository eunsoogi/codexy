use std::fs;

use serde_json::{Value, json};

use super::{fixture, graph};
use crate::support::{FixtureCommand, TestResult};

pub(crate) fn validate_handoff(
    control: &serde_json::Value,
    pull_request: u64,
    head_seed: &str,
) -> TestResult<std::process::Output> {
    validate_handoff_with_state(control, pull_request, head_seed, |_| Ok(()))
}

pub(crate) fn validate_handoff_with_state<F>(
    control: &Value,
    pull_request: u64,
    head_seed: &str,
    mutate: F,
) -> TestResult<std::process::Output>
where
    F: FnOnce(&mut Value) -> TestResult<()>,
{
    let temporary = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temporary.path())?;
    let base_seed = control["final_disposition"]["base_oid"]
        .as_str()
        .ok_or("final disposition base")?;
    let (mut control, _, current_base) = repository.prepare(control, base_seed, base_seed)?;
    let root_repair = control["post_cap_re_review"]["reason"].as_str()
        == Some("in_scope_contract_root_repair");
    let head = repository.resolve(head_seed, root_repair, false, false)?;
    let issue = control["issue_number"].as_u64().ok_or("handoff issue")?;
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
        &head,
        finding_id,
    )?;
    if control["final_disposition"].get("authority").is_none() {
        control["final_disposition"]["authority"] = json!({
            "locator": fixture::locator(issue, pull_request)
        });
    }
    let handoff = temporary.path().join("handoff.md");
    let state = temporary.path().join("state.json");
    fs::write(&handoff, "PASS on the exact current head.\n")?;
    let mut pr_state = json!({
            "repository": "eunsoogi/codexy",
            "number": pull_request,
            "url": format!("https://github.com/eunsoogi/codexy/pull/{pull_request}"),
            "baseRefName": "main",
            "baseRefOid": current_base,
            "capture": {
                "provider": "github",
                "method": "graphql",
                "authenticated": true,
                "owningIssue": {
                    "repository": "eunsoogi/codexy",
                    "number": issue,
                    "url": format!("https://github.com/eunsoogi/codexy/issues/{issue}"),
                    "association": "owner-assignment"
                }
            },
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "headRefOid": head,
            "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": []},
            "reviewProfile": control["profile"].clone(),
            "reviewControl": control
    });
    mutate(&mut pr_state)?;
    fs::write(&state, serde_json::to_vec(&pr_state)?)?;
    let mut command = FixtureCommand::new(env!("CARGO_BIN_EXE_codexy-validate"));
    fixture::configure(&mut command, &fixture);
    command.env_path("CODEXY_REPO_ROOT", &repository.path);
    Ok(command
        .args(["--check-completion-handoff", "--handoff-file"])
        .arg_path(&handoff)
        .args(["--pr-state-file"])
        .arg_path(&state)
        .args(["--plugin-root"])
        .arg_path(codexy_runtime::paths::repository_root().join("plugins/codexy"))
        .output()?)
}
