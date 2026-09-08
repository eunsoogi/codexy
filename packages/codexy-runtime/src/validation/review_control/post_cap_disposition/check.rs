use std::collections::HashSet;

use serde_json::Value;

use super::super::pre_pr::{object, reject_unknown, text};
use super::{KINDS, SCHEMA, capture};

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
            &["id", "path", "kind", "requiredDisposition"],
            "finding disposition record",
        )?;
        let id = text(finding, "id", "finding disposition record")?;
        if !ids.insert(id) {
            return Err("finding disposition ids must be unique".into());
        }
        let path = text(finding, "path", "finding disposition record")?;
        if let Some(kind) = finding.get("kind")
            && kind.as_str().is_none_or(str::is_empty)
        {
            return Err("finding disposition record kind must be non-empty".into());
        }
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
