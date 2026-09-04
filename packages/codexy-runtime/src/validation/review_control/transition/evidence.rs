use std::{collections::HashSet, path::Path, process::Command};

use serde_json::{Map, Value};

use super::super::snapshot;

struct RootRepair<'a> {
    repository_root: &'a Path,
    previous_base: &'a str,
    current_base: &'a str,
    prior_delta: &'a Map<String, Value>,
    change: &'a Map<String, Value>,
    from: &'a str,
    evidence: &'a str,
}

pub(super) fn check(
    repository_root: &Path,
    previous: &Value,
    current: &Value,
    control: &Map<String, Value>,
    history: &[Value],
) -> Result<(), String> {
    let post_cap = control
        .get("post_cap_re_review")
        .and_then(Value::as_object)
        .ok_or_else(|| "review control transition requires post-cap evidence".to_owned())?;
    let reason = required_text(post_cap, "reason", "post-cap evidence")?;
    let prior_delta = history
        .get(1)
        .and_then(Value::as_object)
        .ok_or_else(|| "review control transition requires a prior delta event".to_owned())?;
    let prior_delta_head = required_text(prior_delta, "reviewed_head", "prior delta event")?;
    let current_object = current
        .as_object()
        .ok_or_else(|| "current PR snapshot must be an object".to_owned())?;
    let previous_object = previous
        .as_object()
        .ok_or_else(|| "previous PR snapshot must be an object".to_owned())?;
    let current_head = snapshot::required_text(current_object, "headRefOid", "current")?;
    let previous_base = snapshot::required_text(previous_object, "baseRefOid", "previous")?;
    let current_base = snapshot::required_text(current_object, "baseRefOid", "current")?;
    let change = post_cap
        .get("qualifying_change")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "review control transition requires qualifying change evidence".to_owned()
        })?;
    let from = required_text(change, "from_head", "qualifying change")?;
    let evidence = required_text(change, "evidence_commit", "qualifying change")?;
    let to = required_text(change, "to_head", "qualifying change")?;
    if from != prior_delta_head || to != current_head {
        return Err("review control transition qualifying change evidence has stale heads".into());
    }
    for (value, label) in [
        (from, "prior delta head"),
        (evidence, "evidence commit"),
        (to, "current head"),
    ] {
        require_commit(value, label)?;
    }
    if evidence == from || evidence == to {
        return Err(
            "review control transition evidence commit must be an intermediate commit".into(),
        );
    }
    check_ancestor(repository_root, from, evidence, "prior delta to evidence")?;
    check_ancestor(repository_root, evidence, to, "evidence to current head")?;

    match reason {
        "mandatory_base_integration" => check_base_integration(
            repository_root,
            previous_base,
            current_base,
            evidence,
            change,
        ),
        "in_scope_contract_root_repair" => check_root_repair(&RootRepair {
            repository_root,
            previous_base,
            current_base,
            prior_delta,
            change,
            from,
            evidence,
        }),
        _ => Err("review control transition post-cap reason is not eligible".into()),
    }
}

fn check_base_integration(
    repository_root: &Path,
    previous_base: &str,
    current_base: &str,
    evidence: &str,
    change: &Map<String, Value>,
) -> Result<(), String> {
    if previous_base == current_base {
        return Err("mandatory base integration must change baseRefOid".into());
    }
    check_ancestor(
        repository_root,
        previous_base,
        current_base,
        "previous base to current base",
    )?;
    check_ancestor(
        repository_root,
        current_base,
        evidence,
        "current base to integration evidence",
    )?;
    if change
        .get("finding_ids")
        .and_then(Value::as_array)
        .is_some_and(|ids| !ids.is_empty())
    {
        return Err("mandatory base integration must not bind root findings".into());
    }
    Ok(())
}

fn check_root_repair(context: &RootRepair<'_>) -> Result<(), String> {
    if context.previous_base != context.current_base {
        return Err("contract/root repair must not change baseRefOid".into());
    }
    if required_text(context.prior_delta, "terminal_result", "prior delta event")? != "BLOCK" {
        return Err("contract/root repair requires a prior BLOCK delta".into());
    }
    let findings = context
        .prior_delta
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "prior delta event must list unresolved findings".to_owned())?;
    if findings.is_empty() {
        return Err("contract/root repair requires prior delta findings".into());
    }
    let expected = finding_ids(findings, "prior delta findings")?;
    let actual = context
        .change
        .get("finding_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "contract/root repair must bind finding ids".to_owned())?;
    if expected != string_ids(actual, "qualifying change finding ids")? {
        return Err("qualifying change evidence is not linked to the prior findings".into());
    }
    require_changed_tree(context.repository_root, context.from, context.evidence)
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

fn require_commit(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("review control {label} must be a commit SHA"));
    }
    Ok(())
}

fn check_ancestor(
    repository_root: &Path,
    ancestor: &str,
    descendant: &str,
    label: &str,
) -> Result<(), String> {
    let status = Command::new("git")
        .current_dir(repository_root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .status()
        .map_err(|error| format!("review control cannot verify {label}: {error}"))?;
    if !status.success() {
        return Err(format!(
            "review control qualifying change is not in repository ancestry: {label}"
        ));
    }
    Ok(())
}

fn require_changed_tree(repository_root: &Path, from: &str, to: &str) -> Result<(), String> {
    let status = Command::new("git")
        .current_dir(repository_root)
        .args(["diff", "--quiet", from, to, "--"])
        .status()
        .map_err(|error| {
            format!("review control cannot inspect qualifying repair diff: {error}")
        })?;
    match status.code() {
        Some(1) => Ok(()),
        Some(0) => Err("contract/root repair evidence must change the reviewed tree".into()),
        _ => Err("review control cannot inspect qualifying repair diff".into()),
    }
}

fn finding_ids(findings: &[Value], label: &str) -> Result<HashSet<String>, String> {
    let mut ids = HashSet::new();
    for finding in findings {
        let object = finding
            .as_object()
            .ok_or_else(|| format!("{label} must contain finding objects"))?;
        let id = required_text(object, "id", label)?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{label} must contain unique finding ids"));
        }
    }
    Ok(ids)
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
