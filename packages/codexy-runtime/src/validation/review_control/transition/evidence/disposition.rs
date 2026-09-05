use std::{collections::HashSet, path::Path};

use serde_json::{Map, Value};

use super::paths;
use crate::validation::review_control::post_cap_disposition;

pub(super) struct Context<'a> {
    pub(super) repository_root: &'a Path,
    pub(super) previous_base: &'a str,
    pub(super) current_base: &'a str,
    pub(super) current: &'a Map<String, Value>,
    pub(super) prior_delta: &'a Map<String, Value>,
    pub(super) change: &'a Map<String, Value>,
    pub(super) from: &'a str,
    pub(super) evidence: &'a str,
}

pub(super) fn check(context: &Context<'_>) -> Result<(), String> {
    let Context {
        repository_root,
        previous_base,
        current_base,
        current,
        prior_delta,
        change,
        from,
        evidence,
    } = context;
    if previous_base != current_base {
        return Err("authenticated finding disposition must not change baseRefOid".into());
    }
    if required_text(prior_delta, "terminal_result", "prior delta event")? != "BLOCK" {
        return Err("authenticated finding disposition requires a prior BLOCK delta".into());
    }
    let prior_findings = prior_delta
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .filter(|findings| !findings.is_empty())
        .ok_or_else(|| {
            "authenticated finding disposition requires prior delta findings".to_owned()
        })?;
    let finding_ids = change
        .get("finding_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "authenticated finding disposition must bind finding ids".to_owned())?;
    if finding_ids.len() != prior_findings.len() {
        return Err(
            "authenticated finding disposition must cover every prior delta finding".into(),
        );
    }
    let disposition = change
        .get("finding_disposition")
        .ok_or_else(|| "authenticated finding disposition must bind its source".to_owned())?;
    post_cap_disposition::check(disposition)?;
    let disposition = disposition
        .as_object()
        .ok_or_else(|| "finding disposition source must be an object".to_owned())?;
    bind_target_identity(
        disposition,
        current,
        current_base,
        required_text(current, "repository", "current")?,
    )?;
    let ci = disposition
        .get("sources")
        .and_then(Value::as_object)
        .and_then(|sources| sources.get("currentHeadCi"))
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition must bind current-head CI facts".to_owned())?;
    if ci.get("repository").and_then(Value::as_str)
        != current.get("repository").and_then(Value::as_str)
        || ci.get("pullRequest") != current.get("number")
        || ci.get("baseRefOid").and_then(Value::as_str) != Some(current_base)
        || ci.get("headRefOid") != current.get("headRefOid")
        || ci.get("complete") != Some(&Value::Bool(true))
    {
        return Err("current-head CI evidence is stale or bound to the wrong PR".into());
    }
    let records = disposition
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "finding disposition must list retained findings".to_owned())?;
    if records.len() != prior_findings.len() {
        return Err("finding disposition must cover every prior delta finding exactly once".into());
    }
    maintainer_kind(disposition, prior_delta)?;
    let classified =
        post_cap_disposition::derive(&Value::Object(disposition.clone()), prior_delta)?;
    let classified_source = classified.0;
    let classified_records = classified_source
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "finding disposition classification lacks findings".to_owned())?;
    let mut seen = HashSet::new();
    for ((prior, id), record) in prior_findings.iter().zip(finding_ids).zip(records) {
        let prior = prior
            .as_object()
            .ok_or_else(|| "prior delta findings must contain objects".to_owned())?;
        let record = record
            .as_object()
            .ok_or_else(|| "finding disposition records must contain objects".to_owned())?;
        let prior_id = required_text(prior, "id", "prior delta finding")?;
        let prior_path = required_text(prior, "path", "prior delta finding")?;
        if id.as_str() != Some(prior_id)
            || required_text(record, "id", "finding disposition record")? != prior_id
            || required_text(record, "path", "finding disposition record")? != prior_path
            || !seen.insert(prior_id)
        {
            return Err(
                "finding disposition does not preserve exact prior finding ID/path order".into(),
            );
        }
        let expected = classified_records
            .iter()
            .find(|record| {
                record.get("id").and_then(Value::as_str) == Some(prior_id)
                    && record.get("path").and_then(Value::as_str) == Some(prior_path)
            })
            .and_then(|record| record.get("requiredDisposition"))
            .and_then(Value::as_str)
            .ok_or_else(|| "finding disposition classification lost a prior finding".to_owned())?;
        if required_text(record, "requiredDisposition", "finding disposition record")? != expected {
            return Err(
                "finding disposition reclassifies a prior finding without its required evidence"
                    .into(),
            );
        }
        if expected == "code_repair" {
            paths::check_with_label(
                repository_root,
                from,
                evidence,
                std::slice::from_ref(&Value::Object(prior.clone())),
                "authenticated finding disposition code repair",
            )?;
        }
    }
    if seen.len() != prior_findings.len() {
        return Err("finding disposition contains duplicate findings".into());
    }
    if !records.iter().any(|record| {
        record.get("requiredDisposition").and_then(Value::as_str) == Some("code_repair")
    }) {
        return Err(
            "authenticated finding disposition must retain actual code-repair evidence".into(),
        );
    }
    Ok(())
}

fn bind_target_identity(
    disposition: &Map<String, Value>,
    current: &Map<String, Value>,
    current_base: &str,
    current_repository: &str,
) -> Result<(), String> {
    if disposition.get("repository").and_then(Value::as_str) != Some(current_repository) {
        return Err("finding disposition changes repository identity".into());
    }
    let issue = disposition
        .get("owningIssue")
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition must bind owning issue".to_owned())?;
    let current_issue = current
        .get("capture")
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("owningIssue"))
        .and_then(Value::as_object)
        .ok_or_else(|| "current PR snapshot must bind owning issue".to_owned())?;
    if issue.get("repository") != current_issue.get("repository")
        || issue.get("number") != current_issue.get("number")
        || issue.get("url") != current_issue.get("url")
    {
        return Err("finding disposition changes owning issue identity".into());
    }
    let pull = disposition
        .get("pullRequest")
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition must bind pull request".to_owned())?;
    if pull.get("repository").and_then(Value::as_str) != Some(current_repository)
        || pull.get("number") != current.get("number")
        || pull.get("baseRefOid").and_then(Value::as_str) != Some(current_base)
        || pull.get("headRefOid") != current.get("headRefOid")
    {
        return Err("finding disposition changes pull request, base, or head identity".into());
    }
    Ok(())
}

fn maintainer_kind<'a>(
    disposition: &'a Map<String, Value>,
    prior_delta: &Map<String, Value>,
) -> Result<Option<(&'a str, &'a str)>, String> {
    let decision = disposition
        .get("sources")
        .and_then(Value::as_object)
        .and_then(|sources| sources.get("maintainerDecision"))
        .and_then(Value::as_object)
        .and_then(|source| source.get("decision"))
        .and_then(Value::as_object)
        .ok_or_else(|| "finding disposition must bind maintainer decision facts".to_owned())?;
    if decision.get("accepted") != Some(&Value::Bool(true)) {
        return Err("maintainer policy disposition must be explicitly accepted".into());
    }
    let prior_reviewer = prior_delta
        .get("reviewer")
        .and_then(Value::as_object)
        .ok_or_else(|| "prior delta event must bind reviewer facts".to_owned())?;
    if required_text(decision, "reviewer", "maintainer decision")?
        != required_text(prior_reviewer, "name", "prior delta reviewer")?
    {
        return Err("maintainer decision does not bind the prior delta reviewer".into());
    }
    Ok(Some((
        required_text(decision, "findingId", "maintainer decision")?,
        required_text(decision, "path", "maintainer decision")?,
    )))
}

fn required_text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("review control {label} must contain non-empty {key}"))
}
