use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use super::super::super::super::pre_pr::{number, object, reject_unknown, text};

pub(super) struct SuiteProjection {
    pub(super) suites: Vec<Value>,
    pub(super) app_ids: BTreeMap<u64, Option<u64>>,
}

pub(super) fn project_suites(raw: &Value, head: &str) -> Result<SuiteProjection, String> {
    let pages = raw
        .as_array()
        .filter(|pages| !pages.is_empty())
        .ok_or_else(|| "authenticated check-suite inventory is missing".to_owned())?;
    let mut total_count = None;
    let mut suites = Vec::new();
    for page in pages {
        let page = object(Some(page), "check-suite inventory page")?;
        reject_unknown(
            page,
            &["total_count", "check_suites"],
            "check-suite inventory page",
        )?;
        let page_total = page
            .get("total_count")
            .and_then(Value::as_u64)
            .ok_or("check-suite inventory page must contain total_count")?;
        if total_count.is_some_and(|total| total != page_total) {
            return Err("check-suite inventory pages disagree on total_count".into());
        }
        total_count = Some(page_total);
        let page_suites = page
            .get("check_suites")
            .and_then(Value::as_array)
            .ok_or("check-suite inventory page must contain check_suites")?;
        suites.extend(page_suites);
    }
    let total_count = total_count.ok_or("check-suite inventory has no pages")?;
    if total_count == 0 || suites.len() as u64 != total_count {
        return Err("authenticated check-suite inventory is incomplete".into());
    }
    let mut ids = BTreeSet::new();
    let mut app_ids = BTreeMap::new();
    let suites = suites
        .into_iter()
        .map(|suite| project_suite(suite, head, &mut ids, &mut app_ids))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SuiteProjection { suites, app_ids })
}

fn project_suite(
    raw: &Value,
    head: &str,
    ids: &mut BTreeSet<u64>,
    app_ids: &mut BTreeMap<u64, Option<u64>>,
) -> Result<Value, String> {
    let suite = object(Some(raw), "check suite")?;
    let id = number(suite, "id", "check suite")?;
    if !ids.insert(id) {
        return Err("check-suite inventory contains duplicate identities".into());
    }
    let suite_head = text(suite, "head_sha", "check suite")?;
    if suite_head != head {
        return Err("check-suite inventory contains a different head".into());
    }
    let status = text(suite, "status", "check suite")?;
    let conclusion = text(suite, "conclusion", "check suite")?;
    if status != "completed" || !matches!(conclusion, "success" | "neutral" | "skipped") {
        return Err("check-suite inventory contains a non-terminal-success suite".into());
    }
    let app_id = optional_id(suite, "app", "check suite")?;
    app_ids.insert(id, app_id);
    Ok(json!({
        "id": id,
        "headSha": suite_head,
        "status": status,
        "conclusion": conclusion,
        "appId": app_id
    }))
}

fn optional_id(map: &Map<String, Value>, key: &str, label: &str) -> Result<Option<u64>, String> {
    let value = map
        .get(key)
        .ok_or_else(|| format!("{label} must contain {key}"))?;
    if value.is_null() {
        return Ok(None);
    }
    let value = object(Some(value), label)?;
    Ok(Some(number(value, "id", label)?))
}
