use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::super::super::super::pre_pr::{number, object, reject_unknown, text};
use super::suites::SuiteProjection;

pub(super) struct InventoryProjection {
    pub(super) total_count: u64,
    pub(super) check_runs: Vec<Value>,
    pub(super) names: BTreeSet<String>,
}

pub(super) fn project_inventory(
    raw: &Value,
    head: &str,
    suites: &SuiteProjection,
) -> Result<InventoryProjection, String> {
    let pages = raw
        .as_array()
        .filter(|pages| !pages.is_empty())
        .ok_or_else(|| "authenticated expected check-run inventory is missing".to_owned())?;
    let mut total_count = None;
    let mut check_runs = Vec::new();
    for page in pages {
        let page = object(Some(page), "expected check-run inventory page")?;
        reject_unknown(
            page,
            &["total_count", "check_runs"],
            "expected check-run inventory page",
        )?;
        let page_total = page
            .get("total_count")
            .and_then(Value::as_u64)
            .ok_or("expected check-run inventory page must contain total_count")?;
        if total_count.is_some_and(|total| total != page_total) {
            return Err("expected check-run inventory pages disagree on total_count".into());
        }
        total_count = Some(page_total);
        let runs = page
            .get("check_runs")
            .and_then(Value::as_array)
            .ok_or("expected check-run inventory page must contain check_runs")?;
        check_runs.extend(runs);
    }
    let total_count = total_count.ok_or("expected check-run inventory has no pages")?;
    if total_count == 0 || check_runs.len() as u64 != total_count {
        return Err("authenticated expected check-run inventory is incomplete".into());
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let check_runs = check_runs
        .into_iter()
        .map(|run| project_check_run(run, head, &mut ids, &mut names))
        .collect::<Result<Vec<_>, _>>()?;
    for run in &check_runs {
        let suite_id = run
            .get("checkSuiteId")
            .and_then(Value::as_u64)
            .ok_or("expected check run must bind a check suite")?;
        let suite_app = suites
            .app_ids
            .get(&suite_id)
            .ok_or("expected check run is not bound to an authenticated check suite")?;
        if let Some(suite_app) = *suite_app {
            if run.get("appId") != Some(&json!(suite_app)) {
                return Err("expected check run and check suite have different apps".into());
            }
        }
    }
    Ok(InventoryProjection {
        total_count,
        check_runs,
        names,
    })
}

fn project_check_run(
    raw: &Value,
    head: &str,
    ids: &mut BTreeSet<u64>,
    names: &mut BTreeSet<String>,
) -> Result<Value, String> {
    let run = object(Some(raw), "expected check run")?;
    let id = number(run, "id", "expected check run")?;
    if !ids.insert(id) {
        return Err("expected check-run inventory contains duplicate identities".into());
    }
    let run_head = text(run, "head_sha", "expected check run")?;
    if run_head != head {
        return Err("expected check-run inventory contains a different head".into());
    }
    if text(run, "status", "expected check run")? != "completed"
        || text(run, "conclusion", "expected check run")? != "success"
    {
        return Err("expected check-run inventory contains a non-terminal-success run".into());
    }
    let name = text(run, "name", "expected check run")?;
    if !names.insert(name.to_owned()) {
        return Err("expected check-run inventory contains duplicate checks".into());
    }
    let app_id = optional_id(run, "app", "expected check run")?;
    let check_suite_id = optional_id(run, "check_suite", "expected check run")?;
    Ok(json!({
        "id": id,
        "name": name,
        "headSha": run_head,
        "status": "completed",
        "conclusion": "success",
        "appId": app_id,
        "checkSuiteId": check_suite_id
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

pub(super) fn project_required_status_checks(
    raw: &Value,
    base_name: &str,
    rollup: &[Value],
    inventory: &InventoryProjection,
) -> Result<Value, String> {
    let response = object(Some(raw), "required status checks response")?;
    if response.get("message").is_some() {
        return Err(
            "required status checks response is not an authenticated protection response".into(),
        );
    }
    let Some(raw_required) = response.get("required_status_checks") else {
        return Ok(empty_required(base_name));
    };
    if raw_required.is_null() {
        return Ok(empty_required(base_name));
    }
    let required = object(Some(raw_required), "required status checks")?;
    let contexts = required
        .get("contexts")
        .and_then(Value::as_array)
        .ok_or("required status checks response must contain contexts")?
        .iter()
        .map(|context| {
            context
                .as_str()
                .filter(|context| !context.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| "required status check context must be non-empty text".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let checks = required
        .get("checks")
        .and_then(Value::as_array)
        .ok_or("required status checks response must contain checks")?
        .iter()
        .map(|check| {
            let check = object(Some(check), "required status check")?;
            let context = text(check, "context", "required status check")?.to_owned();
            let app_id = check.get("app_id").map(|value| {
                value
                    .as_u64()
                    .ok_or_else(|| "required status check app_id must be numeric".to_owned())
            });
            Ok((context, app_id.transpose()?))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut seen_contexts = BTreeSet::new();
    for context in &contexts {
        if !seen_contexts.insert(context) {
            return Err("required status checks contain duplicate contexts".into());
        }
        if !inventory_contains(context, None, rollup, inventory) {
            return Err("required status check is absent from exact check-run inventory".into());
        }
    }
    let mut projected_checks = Vec::with_capacity(checks.len());
    let mut seen_checks = BTreeSet::new();
    for (context, app_id) in checks {
        let identity = format!("{context}:{app_id:?}");
        if !seen_checks.insert(identity) {
            return Err("required status checks contain duplicate checks".into());
        }
        if !inventory_contains(&context, app_id, rollup, inventory) {
            return Err(
                "required status check with its app is absent from exact check-run inventory"
                    .into(),
            );
        }
        let mut projected = Map::new();
        projected.insert("context".into(), Value::String(context));
        if let Some(app_id) = app_id {
            projected.insert("appId".into(), json!(app_id));
        }
        projected_checks.push(Value::Object(projected));
    }
    let state = (contexts.is_empty() && projected_checks.is_empty())
        .then_some("known_empty")
        .unwrap_or("configured");
    Ok(json!({
        "baseRefName": base_name,
        "state": state,
        "contexts": contexts,
        "checks": projected_checks
    }))
}

fn empty_required(base_name: &str) -> Value {
    json!({
        "baseRefName": base_name,
        "state": "known_empty",
        "contexts": [],
        "checks": []
    })
}

fn inventory_contains(
    context: &str,
    app_id: Option<u64>,
    rollup: &[Value],
    inventory: &InventoryProjection,
) -> bool {
    let Some(name) = rollup.iter().find_map(|check| {
        let check = check.as_object()?;
        let name = check.get("name")?.as_str()?;
        let workflow = check.get("workflowName")?.as_str()?;
        (context == name
            || context == format!("{workflow}/{name}")
            || context == format!("{workflow} / {name}")
            || context == format!("{workflow}: {name}"))
        .then_some(name)
    }) else {
        return false;
    };
    inventory.check_runs.iter().any(|run| {
        run.get("name").and_then(Value::as_str) == Some(name)
            && app_id.is_none_or(|app_id| run.get("appId") == Some(&json!(app_id)))
    })
}
