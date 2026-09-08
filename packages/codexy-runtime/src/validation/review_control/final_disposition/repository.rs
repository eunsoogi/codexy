use std::{collections::HashSet, path::Path, process::Command};

use serde_json::{Map, Value};

use super::super::snapshot;
use super::fields::{oid, section, section_ids, text};

pub(super) fn check(
    repository_root: &Path,
    previous: &Value,
    current: &Value,
    control: &Map<String, Value>,
) -> Result<(), String> {
    let previous_object = previous
        .as_object()
        .ok_or_else(|| "previous PR snapshot must be an object".to_owned())?;
    let current_object = current
        .as_object()
        .ok_or_else(|| "current PR snapshot must be an object".to_owned())?;
    let previous_base = snapshot::required_oid(previous_object, "baseRefOid", "previous")?;
    let current_base = snapshot::required_oid(current_object, "baseRefOid", "current")?;
    if previous_base != current_base {
        return Err("final disposition must preserve baseRefOid".into());
    }
    let current_head = snapshot::required_oid(current_object, "headRefOid", "current")?;
    let history = control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .ok_or_else(|| "final disposition requires terminal review history".to_owned())?;
    let source_head = history
        .get(2)
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition requires a third review event".to_owned())?
        .get("reviewed_head")
        .and_then(Value::as_str)
        .ok_or_else(|| "third review event must contain reviewed_head".to_owned())?;
    if source_head == current_head {
        return Ok(());
    }
    check_repaired_head(repository_root, current_head, history, control, source_head)
}

pub(super) fn check_handoff(
    repository_root: &Path,
    current: &Value,
    control: &Map<String, Value>,
) -> Result<(), String> {
    let current_object = current
        .as_object()
        .ok_or_else(|| "final disposition handoff PR snapshot must be an object".to_owned())?;
    let current_head = snapshot::required_oid(current_object, "headRefOid", "current")?;
    let history = control
        .get("terminal_review_history")
        .and_then(Value::as_array)
        .ok_or_else(|| "final disposition handoff requires terminal review history".to_owned())?;
    let source_head = history
        .get(2)
        .and_then(Value::as_object)
        .ok_or_else(|| "final disposition handoff requires a third review event".to_owned())?
        .get("reviewed_head")
        .and_then(Value::as_str)
        .ok_or_else(|| "third review event must contain reviewed_head".to_owned())?;
    if source_head == current_head {
        return Ok(());
    }
    check_repaired_head(repository_root, current_head, history, control, source_head)
}

fn check_repaired_head(
    repository_root: &Path,
    current_head: &str,
    history: &[Value],
    control: &Map<String, Value>,
    source_head: &str,
) -> Result<(), String> {
    let repair = section(
        control
            .get("final_disposition")
            .and_then(Value::as_object)
            .and_then(|value| value.get("source_repair"))
            .ok_or_else(|| "repaired final head requires source repair evidence".to_owned())?,
        "source repair",
    )?;
    let evidence = oid(repair, "evidence_commit", "source repair")?;
    if evidence == current_head {
        return Err("final disposition repair evidence must precede the current head".into());
    }
    check_ancestor(
        repository_root,
        source_head,
        evidence,
        "third head to repair evidence",
    )?;
    check_ancestor(
        repository_root,
        evidence,
        current_head,
        "repair evidence to current head",
    )?;
    let third_findings = history
        .get(2)
        .ok_or_else(|| "final disposition requires a third review event".to_owned())?
        .get("unresolved_findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "third review event must list findings".to_owned())?;
    let repair_ids = section_ids(repair, "source repair")?;
    let selected = third_findings
        .iter()
        .filter(|finding| {
            finding
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| repair_ids.iter().any(|selected| selected == id))
        })
        .collect::<Vec<_>>();
    if selected.len() != repair_ids.len() {
        return Err("source repair finding coverage does not bind third-review findings".into());
    }
    check_repair_paths(
        repository_root,
        source_head,
        evidence,
        current_head,
        &selected,
    )
}

fn check_repair_paths(
    repository_root: &Path,
    from: &str,
    to: &str,
    current: &str,
    findings: &[&Value],
) -> Result<(), String> {
    let evidence_changed = changed_paths(repository_root, from, to)?;
    let current_changed = changed_paths(repository_root, from, current)?;
    if evidence_changed.is_empty() || current_changed.is_empty() {
        return Err("final disposition source repair must retain a non-empty final diff".into());
    }
    let mut allowed_paths = HashSet::new();
    for finding in findings {
        let path = text(
            finding
                .as_object()
                .ok_or_else(|| "source repair finding must be an object".to_owned())?,
            "path",
            "source repair finding",
        )?;
        if !is_repository_relative(path) {
            return Err("final disposition repair finding path must be repository-relative".into());
        }
        allowed_paths.insert(path);
    }
    if evidence_changed
        .iter()
        .chain(current_changed.iter())
        .any(|changed| !allowed_paths.contains(changed.as_str()))
    {
        return Err("final disposition source repair changes an out-of-scope path".into());
    }
    for path in allowed_paths {
        if !evidence_changed.iter().any(|changed| changed == path) {
            return Err(format!(
                "final disposition repair evidence does not change finding path: {path}"
            ));
        }
        if !current_changed.iter().any(|changed| changed == path) {
            return Err(format!(
                "final disposition current head does not retain finding path change: {path}"
            ));
        }
    }
    Ok(())
}

fn changed_paths(repository_root: &Path, from: &str, to: &str) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(repository_root)
        .args(["diff", "--name-only", "--no-ext-diff", from, to, "--"])
        .output()
        .map_err(|error| format!("review control cannot inspect final repair diff: {error}"))?;
    if !output.status.success() {
        return Err("final disposition source repair must change the reviewed tree".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

fn is_repository_relative(path: &str) -> bool {
    !path.starts_with('/')
        && !path.contains('\\')
        && !path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
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
            "review control final disposition is not in repository ancestry: {label}"
        ));
    }
    Ok(())
}
