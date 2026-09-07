use serde_json::{Map, Value};

use super::{FIELDS, normalize_log, ownership};
use crate::validation::review_control::external_finding::capture::actions::MAX_LOG_BYTES;
use crate::validation::review_control::pre_pr::{object, reject_unknown, text};

pub(super) fn check_shape(capture: &Map<String, Value>) -> Result<(), String> {
    reject_unknown(
        capture,
        &["provider", "method", "authenticated", "raw"],
        "Actions capture",
    )?;
    if text(capture, "provider", "Actions capture")? != "github"
        || text(capture, "method", "Actions capture")? != "actions"
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err("Actions source is not authenticated GitHub Actions".into());
    }
    let raw = object(capture.get("raw"), "Actions raw capture")?;
    reject_unknown(
        raw,
        &[
            "repository",
            "run",
            "job",
            "step",
            "relation",
            "issueRelation",
            "sourceOwnership",
            "log",
            "projection",
        ],
        "Actions raw capture",
    )?;
    text(raw, "repository", "Actions raw capture")?;
    check_raw_object(
        raw,
        "run",
        &[
            "id",
            "run_attempt",
            "event",
            "workflow_id",
            "path",
            "head_sha",
            "status",
            "conclusion",
        ],
    )?;
    check_raw_object(
        raw,
        "job",
        &[
            "id",
            "name",
            "run_attempt",
            "head_sha",
            "status",
            "conclusion",
        ],
    )?;
    check_raw_object(
        raw,
        "step",
        &[
            "number",
            "name",
            "status",
            "conclusion",
            "started_at",
            "completed_at",
        ],
    )?;
    check_raw_object(raw, "relation", &["repository", "number", "url"])?;
    check_raw_object(
        raw,
        "issueRelation",
        &["repository", "number", "url", "association"],
    )?;
    ownership::check_shape(raw.get("sourceOwnership"))?;
    reject_unknown(
        object(raw.get("projection"), "Actions raw projection")?,
        &FIELDS,
        "Actions raw projection",
    )?;
    let log = text(raw, "log", "Actions raw capture")?;
    if log.len() > MAX_LOG_BYTES || log.contains('\0') || normalize_log(log) != log {
        return Err("Actions job log is missing or exceeds its bound".into());
    }
    Ok(())
}

fn check_raw_object(raw: &Map<String, Value>, key: &str, fields: &[&str]) -> Result<(), String> {
    let value = object(raw.get(key), &format!("Actions raw {key}"))?;
    reject_unknown(value, fields, &format!("Actions raw {key}"))
}
