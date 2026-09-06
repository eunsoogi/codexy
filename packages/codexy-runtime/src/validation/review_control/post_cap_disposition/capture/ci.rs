use std::process::Command;

use serde_json::{Value, json};

use super::super::super::pre_pr::{object, reject_unknown, text};
use super::{Locator, bounded_response};

mod inventory;
mod pull;
mod suites;

use inventory::{project_inventory, project_required_status_checks};
use pull::{oid, project_pull};
use suites::project_suites;

const SCHEMA: &str = "codexy.github-current-head-ci.v1";

pub(super) fn read(locator: &Locator) -> Result<(Value, Value), String> {
    let pull_number = locator.pull_request.to_string();
    let mut pull_command = Command::new("gh");
    pull_command.args([
        "pr",
        "view",
        &pull_number,
        "--repo",
        &locator.repository,
        "--json",
        "number,baseRefName,baseRefOid,headRefName,headRefOid,statusCheckRollup",
    ]);
    let pull = run_json(&mut pull_command, "current-head CI PR")?;
    let pull_object = object(Some(&pull), "current-head CI PR response")?;
    let base_name = text(pull_object, "baseRefName", "current-head CI PR response")?;
    let head = text(pull_object, "headRefOid", "current-head CI PR response")?;

    let protection_endpoint = format!(
        "repos/{}/branches/{}/protection",
        locator.repository, base_name
    );
    let mut protection_command = Command::new("gh");
    protection_command.args(["api", &protection_endpoint]);
    let required_status_checks = run_json(&mut protection_command, "required status checks")?;

    let check_runs_endpoint = format!(
        "repos/{}/commits/{}/check-runs?per_page=100",
        locator.repository, head
    );
    let mut check_runs_command = Command::new("gh");
    check_runs_command
        .args(["api", "--paginate", "--slurp"])
        .arg(&check_runs_endpoint);
    let expected_check_runs = run_json(&mut check_runs_command, "expected check runs")?;

    let check_suites_endpoint = format!(
        "repos/{}/commits/{}/check-suites?per_page=100",
        locator.repository, head
    );
    let mut check_suites_command = Command::new("gh");
    check_suites_command
        .args(["api", "--paginate", "--slurp"])
        .arg(&check_suites_endpoint);
    let check_suites = run_json(&mut check_suites_command, "check suites")?;

    let raw = json!({
        "pullRequest": pull,
        "requiredStatusChecks": {
            "baseRefName": base_name,
            "response": required_status_checks
        },
        "expectedCheckRuns": {
            "headRefOid": head,
            "response": expected_check_runs
        },
        "checkSuites": {
            "headRefOid": head,
            "response": check_suites
        }
    });
    let projection = project(&raw, locator)?;
    Ok((raw, projection))
}

pub(super) fn project(raw: &Value, locator: &Locator) -> Result<Value, String> {
    let response = object(Some(raw), "current-head CI source")?;
    reject_unknown(
        response,
        &[
            "pullRequest",
            "requiredStatusChecks",
            "expectedCheckRuns",
            "checkSuites",
        ],
        "current-head CI source",
    )?;
    let pull = object(response.get("pullRequest"), "current-head CI PR response")?;
    let pull = project_pull(pull, locator)?;
    let required = object(
        response.get("requiredStatusChecks"),
        "current-head CI required-check source",
    )?;
    reject_unknown(
        required,
        &["baseRefName", "response"],
        "current-head CI required-check source",
    )?;
    if text(
        required,
        "baseRefName",
        "current-head CI required-check source",
    )? != pull.base_name
    {
        return Err("current-head CI required-check source changes the base branch".into());
    }
    let expected = object(
        response.get("expectedCheckRuns"),
        "current-head CI expected-check source",
    )?;
    reject_unknown(
        expected,
        &["headRefOid", "response"],
        "current-head CI expected-check source",
    )?;
    let expected_head = oid(expected, "headRefOid")?;
    if expected_head != pull.head {
        return Err("current-head CI expected-check source changes the PR head".into());
    }
    let suites = object(response.get("checkSuites"), "current-head CI suite source")?;
    reject_unknown(
        suites,
        &["headRefOid", "response"],
        "current-head CI suite source",
    )?;
    if text(suites, "headRefOid", "current-head CI suite source")? != pull.head {
        return Err("current-head CI suite source changes the PR head".into());
    }
    let suites = project_suites(
        suites
            .get("response")
            .ok_or("current-head CI suite source is missing its response")?,
        &pull.head,
    )?;
    let inventory = project_inventory(
        expected
            .get("response")
            .ok_or("current-head CI expected-check source is missing its response")?,
        &pull.head,
        &suites,
    )?;
    if pull.names != inventory.names {
        return Err(
            "current-head CI rollup and authenticated check-run inventory have different check names"
                .into(),
        );
    }
    let required = project_required_status_checks(
        required
            .get("response")
            .ok_or("current-head CI required-check source is missing its response")?,
        &pull.base_name,
        &pull.checks,
        &inventory,
    )?;
    Ok(json!({
        "schema": SCHEMA,
        "repository": locator.repository,
        "pullRequest": locator.pull_request,
        "baseRefName": pull.base_name,
        "baseRefOid": pull.base,
        "headRefName": pull.head_name,
        "headRefOid": pull.head,
        "complete": true,
        "checks": pull.checks,
        "requiredStatusChecks": required,
        "expectedCheckRuns": {
            "headRefOid": pull.head,
            "coverage": "registered_check_runs",
            "totalCount": inventory.total_count,
            "checkRuns": inventory.check_runs
        },
        "checkSuites": suites.suites
    }))
}

fn run_json(command: &mut Command, label: &str) -> Result<Value, String> {
    let output = command
        .output()
        .map_err(|error| format!("authenticated GitHub {label} read failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "authenticated GitHub {label} read failed: {}",
            bounded_stderr(&output.stderr)
        ));
    }
    bounded_response(&output.stdout, label)
}

fn bounded_stderr(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .trim()
        .chars()
        .take(512)
        .collect()
}
