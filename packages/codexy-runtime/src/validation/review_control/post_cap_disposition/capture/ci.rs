use std::process::Command;

use serde_json::{Map, Value, json};

use super::super::super::pre_pr::{object, reject_unknown, text};
use super::{Locator, bounded_response};

const SCHEMA: &str = "codexy.github-current-head-ci.v1";

pub(super) fn read(locator: &Locator) -> Result<(Value, Value), String> {
    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            &locator.pull_request.to_string(),
            "--repo",
            &locator.repository,
            "--json",
            "number,baseRefName,baseRefOid,headRefName,headRefOid,statusCheckRollup",
        ])
        .output()
        .map_err(|error| format!("authenticated GitHub current-head CI read failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "authenticated GitHub current-head CI read failed: {}",
            bounded_stderr(&output.stderr)
        ));
    }
    let raw = bounded_response(&output.stdout, "current-head CI")?;
    let projection = project(&raw, locator)?;
    Ok((raw, projection))
}

pub(super) fn project(raw: &Value, locator: &Locator) -> Result<Value, String> {
    let response = object(Some(raw), "current-head CI response")?;
    reject_unknown(
        response,
        &[
            "number",
            "baseRefName",
            "baseRefOid",
            "headRefName",
            "headRefOid",
            "statusCheckRollup",
        ],
        "current-head CI response",
    )?;
    if response.get("number") != Some(&json!(locator.pull_request)) {
        return Err("current-head CI response changes pull request identity".into());
    }
    let base = oid(response, "baseRefOid")?;
    let head = oid(response, "headRefOid")?;
    let base_name = text(response, "baseRefName", "current-head CI response")?;
    let head_name = text(response, "headRefName", "current-head CI response")?;
    let checks = response
        .get("statusCheckRollup")
        .and_then(Value::as_array)
        .filter(|checks| !checks.is_empty())
        .ok_or_else(|| {
            "current-head CI source must contain a complete non-empty check rollup".to_owned()
        })?;
    let mut projected = Vec::with_capacity(checks.len());
    let mut identities = std::collections::HashSet::new();
    for check in checks {
        let check_object = object(Some(check), "current-head CI check")?;
        let kind = text(check_object, "__typename", "current-head CI check")?;
        if kind != "CheckRun" {
            return Err("current-head CI source contains an unsupported check context".into());
        }
        let name = text(check_object, "name", "current-head CI check")?;
        let workflow = text(check_object, "workflowName", "current-head CI check")?;
        let status = text(check_object, "status", "current-head CI check")?;
        let conclusion = text(check_object, "conclusion", "current-head CI check")?;
        if status != "COMPLETED" || conclusion != "SUCCESS" {
            return Err("current-head CI source contains a non-terminal-success check".into());
        }
        if !identities.insert((workflow.to_owned(), name.to_owned())) {
            return Err("current-head CI source contains duplicate checks".into());
        }
        let mut item = Map::new();
        item.insert("type".into(), Value::String(kind.to_owned()));
        item.insert("name".into(), Value::String(name.to_owned()));
        item.insert("workflowName".into(), Value::String(workflow.to_owned()));
        item.insert("status".into(), Value::String(status.to_owned()));
        item.insert("conclusion".into(), Value::String(conclusion.to_owned()));
        if let Some(url) = check_object.get("detailsUrl") {
            item.insert("detailsUrl".into(), url.clone());
        }
        projected.push(Value::Object(item));
    }
    Ok(json!({
        "schema": SCHEMA,
        "repository": locator.repository,
        "pullRequest": locator.pull_request,
        "baseRefName": base_name,
        "baseRefOid": base,
        "headRefName": head_name,
        "headRefOid": head,
        "complete": true,
        "checks": projected
    }))
}

fn oid(object: &Map<String, Value>, key: &str) -> Result<String, String> {
    let value = text(object, key, "current-head CI response")?;
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "current-head CI response {key} must be a commit SHA"
        ));
    }
    Ok(value.to_owned())
}

fn bounded_stderr(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .trim()
        .chars()
        .take(512)
        .collect()
}
