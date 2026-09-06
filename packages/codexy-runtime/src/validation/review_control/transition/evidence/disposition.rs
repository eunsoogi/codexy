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
    post_cap_disposition::check_live_binding(disposition, current, current_base, prior_delta)?;
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
