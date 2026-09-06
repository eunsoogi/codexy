use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::super::super::super::pre_pr::{object, reject_unknown, text};
use super::super::Locator;

pub(super) struct PullProjection {
    pub(super) base_name: String,
    pub(super) base: String,
    pub(super) head_name: String,
    pub(super) head: String,
    pub(super) checks: Vec<Value>,
    pub(super) names: BTreeSet<String>,
}

pub(super) fn project_pull(
    response: &Map<String, Value>,
    locator: &Locator,
) -> Result<PullProjection, String> {
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
        "current-head CI PR response",
    )?;
    if response.get("number") != Some(&json!(locator.pull_request)) {
        return Err("current-head CI response changes pull request identity".into());
    }
    let base = oid(response, "baseRefOid")?;
    let head = oid(response, "headRefOid")?;
    let base_name = text(response, "baseRefName", "current-head CI PR response")?;
    let head_name = text(response, "headRefName", "current-head CI PR response")?;
    let checks = response
        .get("statusCheckRollup")
        .and_then(Value::as_array)
        .filter(|checks| !checks.is_empty())
        .ok_or_else(|| {
            "current-head CI source must contain a complete non-empty check rollup".to_owned()
        })?;
    let mut projected = Vec::with_capacity(checks.len());
    let mut names = BTreeSet::new();
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
        if !names.insert(name.to_owned()) {
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
    Ok(PullProjection {
        base_name: base_name.to_owned(),
        base,
        head_name: head_name.to_owned(),
        head,
        checks: projected,
        names,
    })
}

pub(super) fn oid(object: &Map<String, Value>, key: &str) -> Result<String, String> {
    let value = text(object, key, "current-head CI response")?;
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "current-head CI response {key} must be a commit SHA"
        ));
    }
    Ok(value.to_owned())
}
