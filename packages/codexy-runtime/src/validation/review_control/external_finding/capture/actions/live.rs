use std::process::Command;

use serde_json::{Map, Value, json};

use super::{Locator, MAX_LOG_BYTES, SCHEMA, projection};
use crate::validation::review_control::pre_pr::{object, text};

mod select;
use select::{select_issue, select_job, select_pull};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub(super) fn read_live(locator: &Value, expected_commit: Option<&str>) -> Result<Value, String> {
    read_locator(Locator::from_value(locator)?, expected_commit)
}

pub(super) fn read_live_from_source(
    source: &Value,
    expected_commit: Option<&str>,
) -> Result<Value, String> {
    let source = object(Some(source), "authenticated Actions finding")?;
    read_locator(Locator::from_source(source)?, expected_commit)
}

fn read_locator(locator: Locator, expected_commit: Option<&str>) -> Result<Value, String> {
    let run = api_json(
        &locator,
        &format!(
            "actions/runs/{}/attempts/{}",
            locator.workflow_run, locator.run_attempt
        ),
    )?;
    let run_object = object(Some(&run), "Actions workflow run")?;
    let observed = text(run_object, "head_sha", "Actions workflow run")?.to_owned();
    let jobs = api_json(
        &locator,
        &format!(
            "actions/runs/{}/attempts/{}/jobs?per_page=100",
            locator.workflow_run, locator.run_attempt
        ),
    )?;
    let (job, step, ambiguous_step_boundary) = select_job(&jobs, &locator, &observed)?;
    let pulls = api_json(&locator, &format!("commits/{observed}/pulls?per_page=100"))?;
    let relation = select_pull(&pulls, &locator)?;
    let timeline = api_json(
        &locator,
        &format!("issues/{}/timeline?per_page=100", locator.pull_request),
    )?;
    let issue_relation = select_issue(&timeline, &locator)?;
    let step_object = object(Some(&step), "Actions failed step")?;
    let log = projection::scoped_log(
        &api_log(&locator, &format!("actions/jobs/{}/logs", locator.job))?,
        step_object,
        &locator.repository,
        ambiguous_step_boundary,
    )?;
    let raw = json!({
        "repository": locator.repository,
        "run": minimal_run(run_object)?,
        "job": job,
        "step": step,
        "relation": relation,
        "issueRelation": issue_relation,
        "log": log
    });
    let raw = raw
        .as_object()
        .ok_or_else(|| "Actions raw capture must be an object".to_owned())?;
    let projection = projection::project(raw, &locator, expected_commit)?;
    let capture = json!({
        "provider": "github",
        "method": "actions",
        "authenticated": true,
        "raw": {
            "repository": raw["repository"],
            "run": raw["run"],
            "job": raw["job"],
            "step": raw["step"],
            "relation": raw["relation"],
            "issueRelation": raw["issueRelation"],
            "log": raw["log"],
            "projection": projection.clone()
        }
    });
    let mut source = projection;
    source.insert("schema".into(), Value::String(SCHEMA.into()));
    source.insert("capture".into(), capture);
    Ok(Value::Object(source))
}

fn minimal_run(run: &Map<String, Value>) -> Result<Value, String> {
    let fields = [
        "id",
        "run_attempt",
        "event",
        "workflow_id",
        "path",
        "head_sha",
        "status",
        "conclusion",
    ];
    let mut result = Map::new();
    for field in fields {
        result.insert(
            field.into(),
            run.get(field)
                .cloned()
                .ok_or_else(|| format!("Actions workflow run is missing {field}"))?,
        );
    }
    Ok(Value::Object(result))
}

fn api_json(locator: &Locator, path: &str) -> Result<Value, String> {
    let output = Command::new("gh")
        .args([
            "api",
            "--hostname",
            "github.com",
            "--method",
            "GET",
            "--allow-escape-sequences",
        ])
        .arg(format!("repos/{}/{}", locator.repository, path))
        .output()
        .map_err(|error| format!("authenticated GitHub Actions read failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "authenticated GitHub Actions read failed: {}",
            bounded(&output.stderr)
        ));
    }
    if output.stdout.len() > MAX_RESPONSE_BYTES {
        return Err("authenticated GitHub Actions response is too large".into());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("authenticated GitHub Actions response is invalid: {error}"))
}

fn api_log(locator: &Locator, path: &str) -> Result<String, String> {
    let output = Command::new("gh")
        .args([
            "api",
            "--hostname",
            "github.com",
            "--method",
            "GET",
            "--allow-escape-sequences",
        ])
        .arg(format!("repos/{}/{}", locator.repository, path))
        .output()
        .map_err(|error| format!("authenticated GitHub Actions log read failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "authenticated GitHub Actions log read failed: {}",
            bounded(&output.stderr)
        ));
    }
    if output.stdout.len() > MAX_LOG_BYTES {
        return Err("authenticated GitHub Actions job log is too large".into());
    }
    String::from_utf8(output.stdout)
        .map(|log| log.trim().to_owned())
        .map_err(|_| "authenticated GitHub Actions job log is not UTF-8".into())
}

fn bounded(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim()
        .chars()
        .take(512)
        .collect()
}
