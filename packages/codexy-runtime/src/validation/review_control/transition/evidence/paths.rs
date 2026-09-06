use std::{collections::HashSet, path::Path, process::Command};

use serde_json::{Map, Value};

use super::super::super::external_finding;

pub(super) struct ExternalFindingContext<'a> {
    pub(super) repository_root: &'a Path,
    pub(super) previous_base: &'a str,
    pub(super) current_base: &'a str,
    pub(super) current_repository: &'a str,
    pub(super) prior_delta: &'a Map<String, Value>,
    pub(super) change: &'a Map<String, Value>,
    pub(super) from: &'a str,
    pub(super) evidence: &'a str,
}

pub(super) fn check(
    repository_root: &Path,
    from: &str,
    to: &str,
    findings: &[Value],
) -> Result<(), String> {
    check_with_label(repository_root, from, to, findings, "prior finding")
}

pub(super) fn check_with_label(
    repository_root: &Path,
    from: &str,
    to: &str,
    findings: &[Value],
    label: &str,
) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(repository_root)
        .args(["diff", "--name-only", "--no-ext-diff", from, to, "--"])
        .output()
        .map_err(|error| {
            format!("review control cannot inspect qualifying repair diff: {error}")
        })?;
    if !output.status.success() {
        return Err("review control cannot inspect qualifying repair diff".into());
    }
    let changed = String::from_utf8_lossy(&output.stdout);
    if changed.lines().next().is_none() {
        return Err("contract/root repair evidence must change the reviewed tree".into());
    }
    for finding in findings {
        let object = finding
            .as_object()
            .ok_or_else(|| "prior delta findings must contain finding objects".to_owned())?;
        let path = required_text(object, "path", label)?;
        if !changed.lines().any(|changed| changed == path) {
            return Err(format!(
                "qualifying change evidence is not linked to {label} path: {path}"
            ));
        }
    }
    Ok(())
}

pub(super) fn check_external_finding(context: ExternalFindingContext<'_>) -> Result<(), String> {
    if context.previous_base != context.current_base {
        return Err("authenticated external finding repair must not change baseRefOid".into());
    }
    if required_text(context.prior_delta, "terminal_result", "prior delta event")? != "PASS"
        || !context
            .prior_delta
            .get("unresolved_findings")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    {
        return Err(
            "authenticated external finding repair requires a clean prior PASS delta".into(),
        );
    }
    let source = context
        .change
        .get("external_finding")
        .ok_or_else(|| "authenticated external finding repair must bind its source".to_owned())?;
    let facts = external_finding::check(source)?;
    if facts.repository != context.current_repository {
        return Err("authenticated external finding changes repository identity".into());
    }
    if facts.observed_commit != context.from {
        return Err("authenticated external finding is stale for the prior delta head".into());
    }
    let actual = context
        .change
        .get("finding_ids")
        .ok_or_else(|| "authenticated external finding repair must bind finding ids".to_owned())?;
    let actual = actual
        .as_array()
        .ok_or_else(|| "qualifying change finding ids must be an array".to_owned())?;
    if string_ids(actual, "qualifying change finding ids")?
        != facts.finding_ids.iter().cloned().collect()
    {
        return Err("qualifying change finding ids do not bind the external source".into());
    }
    check_with_label(
        context.repository_root,
        context.from,
        context.evidence,
        &facts.findings,
        "authenticated external finding",
    )
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

fn string_ids(values: &[Value], label: &str) -> Result<HashSet<String>, String> {
    let mut ids = HashSet::new();
    for value in values {
        let id = value
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| format!("{label} must contain non-empty strings"))?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{label} must be unique"));
        }
    }
    Ok(ids)
}
