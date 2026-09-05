use std::collections::HashSet;

use serde_json::{Map, Value, json};

use super::super::pre_pr::{object, text};
use super::check;

pub(super) fn derive(
    source: &Value,
    prior_delta: &Map<String, Value>,
) -> Result<(Value, Vec<Value>), String> {
    check(source)?;
    let source_object = object(Some(source), "authenticated finding disposition")?;
    let prior_findings = prior_delta
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .filter(|findings| !findings.is_empty())
        .ok_or_else(|| "finding disposition requires prior delta findings".to_owned())?;
    let maintainer = source_object
        .get("sources")
        .and_then(Value::as_object)
        .and_then(|sources| sources.get("maintainerDecision"))
        .and_then(Value::as_object)
        .and_then(|decision| decision.get("decision"))
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition source lacks maintainer decision facts".to_owned())?;
    let maintainer_id = text(maintainer, "findingId", "maintainer decision")?;
    let maintainer_path = text(maintainer, "path", "maintainer decision")?;
    if maintainer.get("accepted") != Some(&Value::Bool(true)) {
        return Err("maintainer policy disposition must be explicitly accepted".into());
    }
    let ci = source_object
        .get("sources")
        .and_then(Value::as_object)
        .and_then(|sources| sources.get("currentHeadCi"))
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition source lacks current-head CI facts".to_owned())?;
    if ci.get("complete") != Some(&Value::Bool(true)) {
        return Err("current-head CI source is not complete".into());
    }
    let mut findings = Vec::with_capacity(prior_findings.len());
    let mut ids = Vec::with_capacity(prior_findings.len());
    let mut seen = HashSet::new();
    for finding in prior_findings {
        let finding = object(Some(finding), "prior delta finding")?;
        let id = text(finding, "id", "prior delta finding")?;
        let path = text(finding, "path", "prior delta finding")?;
        let kind = finding
            .get("kind")
            .and_then(Value::as_str)
            .filter(|kind| !kind.is_empty())
            .unwrap_or("unknown");
        if !seen.insert(id) {
            return Err("prior delta findings must contain unique ids".into());
        }
        let required =
            if kind == "policy_difference" && id == maintainer_id && path == maintainer_path {
                "maintainer_accepted_policy_difference"
            } else if kind == "ci_incomplete_observation" && path.starts_with(".github/workflows/")
            {
                "current_head_ci_terminal"
            } else {
                "code_repair"
            };
        findings
            .push(json!({"id": id, "path": path, "kind": kind, "requiredDisposition": required}));
        ids.push(Value::String(id.to_owned()));
    }
    let mut source = source.clone();
    source["findings"] = Value::Array(findings);
    Ok((source, ids))
}
