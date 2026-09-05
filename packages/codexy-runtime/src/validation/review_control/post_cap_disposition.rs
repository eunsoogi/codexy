use std::collections::HashSet;

use serde_json::{Value, json};

use super::pre_pr::{object, reject_unknown, text};

mod capture;

pub(super) const REASON: &str = "authenticated_finding_disposition";
const SCHEMA: &str = "codexy.review-control-finding-disposition.v1";
const KINDS: [&str; 3] = [
    "code_repair",
    "current_head_ci_terminal",
    "maintainer_accepted_policy_difference",
];

pub(super) fn requires_source(control: &Value) -> bool {
    control
        .get("post_cap_re_review")
        .and_then(Value::as_object)
        .and_then(|post_cap| post_cap.get("reason"))
        .and_then(Value::as_str)
        == Some(REASON)
}

pub(super) fn read_live(locator: &Value, expected_head: Option<&str>) -> Result<Value, String> {
    capture::read_live(capture::Locator::from_value(locator)?, expected_head)
}

pub(super) fn validate_locator(locator: &Value, current: &Value) -> Result<(), String> {
    let locator = capture::Locator::from_value(locator)?;
    let current = object(Some(current), "current PR snapshot")?;
    if current.get("repository").and_then(Value::as_str) != Some(locator.repository.as_str())
        || current.get("number").and_then(Value::as_u64) != Some(locator.pull_request)
    {
        return Err("finding disposition locator does not bind the current PR".into());
    }
    let issue = current
        .get("capture")
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("owningIssue"))
        .and_then(Value::as_object)
        .ok_or_else(|| "current PR snapshot must bind owning issue".to_owned())?;
    if issue.get("repository").and_then(Value::as_str) != Some(locator.repository.as_str())
        || issue.get("number").and_then(Value::as_u64) != Some(locator.owning_issue)
    {
        return Err("finding disposition locator does not bind the owning issue".into());
    }
    Ok(())
}

pub(super) fn refresh_live(control: &mut Value, current: Option<&Value>) -> Result<(), String> {
    let (source, findings, expected_head) = {
        let control_object = object(Some(control), "review control state")?;
        let post_cap = object(
            control_object.get("post_cap_re_review"),
            "post-cap evidence",
        )?;
        if text(post_cap, "reason", "post-cap evidence")? != REASON {
            return Err(
                "finding disposition source is only valid for its typed post-cap reason".into(),
            );
        }
        let change = object(post_cap.get("qualifying_change"), "qualifying change")?;
        let source = object(
            change.get("finding_disposition"),
            "authenticated finding disposition",
        )?;
        let findings = source
            .get("findings")
            .cloned()
            .ok_or_else(|| "finding disposition must retain its finding coverage".to_owned())?;
        (
            source.clone(),
            findings,
            text(change, "to_head", "qualifying change")?.to_owned(),
        )
    };
    let locator = source
        .get("locator")
        .ok_or_else(|| "finding disposition must retain its live source locator".to_owned())?;
    if let Some(current) = current {
        validate_locator(locator, current)?;
    }
    let live = read_live(locator, Some(&expected_head))?;
    let mut live = live
        .as_object()
        .ok_or_else(|| "live finding disposition must be an object".to_owned())?
        .clone();
    live.insert("findings".into(), findings);
    check(&Value::Object(live.clone()))?;
    let post_cap = control
        .get_mut("post_cap_re_review")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "post-cap evidence must be an object".to_owned())?;
    let change = post_cap
        .get_mut("qualifying_change")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "qualifying change must be an object".to_owned())?;
    change.insert("finding_disposition".into(), Value::Object(live));
    Ok(())
}

pub(super) fn normalize_producer(
    control: &mut Value,
    source: &Value,
    previous: &Value,
) -> Result<(), String> {
    check(source)?;
    let source = source
        .as_object()
        .ok_or_else(|| "finding disposition source must be an object".to_owned())?;
    let prior = previous
        .get("reviewControl")
        .and_then(Value::as_object)
        .and_then(|control| control.get("terminal_review_history"))
        .and_then(Value::as_array)
        .and_then(|history| history.get(1))
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition producer requires a prior delta event".to_owned())?;
    let prior_findings = prior
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .filter(|findings| !findings.is_empty())
        .ok_or_else(|| "finding disposition producer requires prior delta findings".to_owned())?;
    let maintainer = source
        .get("sources")
        .and_then(Value::as_object)
        .and_then(|sources| sources.get("maintainerDecision"))
        .and_then(Value::as_object)
        .and_then(|decision| decision.get("decision"))
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition source lacks maintainer decision facts".to_owned())?;
    let maintainer_id = text(maintainer, "findingId", "maintainer decision")?;
    let maintainer_path = text(maintainer, "path", "maintainer decision")?;
    let ci = source
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
        if !seen.insert(id) {
            return Err("prior delta findings must contain unique ids".into());
        }
        let required = if id == maintainer_id && path == maintainer_path {
            "maintainer_accepted_policy_difference"
        } else if path.starts_with(".github/workflows/") {
            "current_head_ci_terminal"
        } else {
            "code_repair"
        };
        findings.push(json!({"id": id, "path": path, "requiredDisposition": required}));
        ids.push(Value::String(id.to_owned()));
    }
    let post_cap = object(
        control
            .as_object()
            .and_then(|object| object.get("post_cap_re_review")),
        "post-cap evidence",
    )?;
    let change = object(post_cap.get("qualifying_change"), "qualifying change")?;
    if change.contains_key("finding_disposition") || change.contains_key("finding_ids") {
        return Err(
            "finding disposition producer rejects caller-supplied classification or finding ids"
                .into(),
        );
    }
    let mut source = source.clone();
    source.insert("findings".into(), Value::Array(findings));
    let post_cap = control
        .as_object_mut()
        .and_then(|object| object.get_mut("post_cap_re_review"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "post-cap evidence must be an object".to_owned())?;
    let change = post_cap
        .get_mut("qualifying_change")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "qualifying change must be an object".to_owned())?;
    change.insert("finding_ids".into(), Value::Array(ids));
    change.insert("finding_disposition".into(), Value::Object(source));
    Ok(())
}

pub(super) fn check(value: &Value) -> Result<(), String> {
    let source = object(Some(value), "authenticated finding disposition")?;
    reject_unknown(
        source,
        &[
            "schema",
            "locator",
            "repository",
            "owningIssue",
            "pullRequest",
            "sources",
            "capture",
            "findings",
        ],
        "authenticated finding disposition",
    )?;
    if text(source, "schema", "finding disposition")? != SCHEMA {
        return Err("finding disposition has an unsupported schema".into());
    }
    capture::check(source)?;
    let Some(findings) = source.get("findings") else {
        return Ok(());
    };
    let findings = findings
        .as_array()
        .ok_or_else(|| "finding disposition findings must be an array".to_owned())?;
    let mut ids = HashSet::new();
    for finding in findings {
        let finding = object(Some(finding), "finding disposition record")?;
        reject_unknown(
            finding,
            &["id", "path", "requiredDisposition"],
            "finding disposition record",
        )?;
        let id = text(finding, "id", "finding disposition record")?;
        if !ids.insert(id) {
            return Err("finding disposition ids must be unique".into());
        }
        let path = text(finding, "path", "finding disposition record")?;
        if path.starts_with('/')
            || path.contains('\\')
            || path
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err("finding disposition paths must be repository-relative".into());
        }
        let kind = text(finding, "requiredDisposition", "finding disposition record")?;
        if !KINDS.contains(&kind) {
            return Err("finding disposition kind is unsupported".into());
        }
    }
    Ok(())
}
