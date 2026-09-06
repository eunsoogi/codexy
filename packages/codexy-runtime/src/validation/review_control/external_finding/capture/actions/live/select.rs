use serde_json::{Map, Value, json};

use super::super::Locator;
use crate::validation::review_control::pre_pr::{number, object, text};

pub(super) fn select_job(
    response: &Value,
    locator: &Locator,
    observed: &str,
) -> Result<(Value, Value, bool), String> {
    let jobs = response
        .as_array()
        .or_else(|| response.get("jobs").and_then(Value::as_array))
        .ok_or_else(|| "Actions jobs response is invalid".to_owned())?;
    let matches = jobs
        .iter()
        .filter(|job| job.get("id").and_then(Value::as_u64) == Some(locator.job))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("Actions exact-attempt jobs response does not identify one job".into());
    }
    let job = object(Some(matches[0]), "Actions job")?;
    let steps = job
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| "Actions job steps are unavailable".to_owned())?;
    let matching_steps = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| {
            step.get("name").and_then(Value::as_str) == Some(locator.step_name.as_str())
        })
        .collect::<Vec<_>>();
    if matching_steps.len() != 1 {
        return Err("Actions job does not identify one failed step".into());
    }
    let (step_index, step_value) = matching_steps[0];
    let step = object(Some(step_value), "Actions failed step")?;
    let ambiguous_step_boundary = adjacent_step_boundary_is_ambiguous(steps, step_index, step)?;
    if text(job, "name", "Actions job")? != locator.job_name
        || text(job, "head_sha", "Actions job")? != observed
        || number(job, "run_attempt", "Actions job")? != locator.run_attempt
    {
        return Err("Actions job identity does not match the exact run attempt".into());
    }
    let mut selected = Map::new();
    for field in [
        "id",
        "name",
        "run_attempt",
        "head_sha",
        "status",
        "conclusion",
    ] {
        selected.insert(
            field.into(),
            job.get(field)
                .cloned()
                .ok_or_else(|| format!("Actions job is missing {field}"))?,
        );
    }
    let mut selected_step = Map::new();
    for field in [
        "number",
        "name",
        "status",
        "conclusion",
        "started_at",
        "completed_at",
    ] {
        selected_step.insert(
            field.into(),
            step.get(field)
                .cloned()
                .ok_or_else(|| format!("Actions step is missing {field}"))?,
        );
    }
    Ok((
        Value::Object(selected),
        Value::Object(selected_step),
        ambiguous_step_boundary,
    ))
}

fn adjacent_step_boundary_is_ambiguous(
    steps: &[Value],
    selected_index: usize,
    selected: &Map<String, Value>,
) -> Result<bool, String> {
    let started_at = timestamp_second(text(selected, "started_at", "Actions failed step")?)?;
    let completed_at = timestamp_second(text(selected, "completed_at", "Actions failed step")?)?;
    if selected_index > 0 {
        let previous = object(Some(&steps[selected_index - 1]), "Actions previous step")?;
        if timestamp_second(text(previous, "completed_at", "Actions previous step")?)? == started_at
        {
            return Ok(true);
        }
    }
    if selected_index + 1 < steps.len() {
        let next = object(Some(&steps[selected_index + 1]), "Actions next step")?;
        if timestamp_second(text(next, "started_at", "Actions next step")?)? == completed_at {
            return Ok(true);
        }
    }
    Ok(false)
}

fn timestamp_second(value: &str) -> Result<&str, String> {
    if !value.ends_with('Z') || !value.contains('T') {
        return Err("Actions step timestamps must be UTC ISO-8601 values".into());
    }
    Ok(value
        .split_once('.')
        .map_or(value, |(second, _)| second)
        .trim_end_matches('Z'))
}

pub(super) fn select_pull(response: &Value, locator: &Locator) -> Result<Value, String> {
    let pulls = response
        .as_array()
        .ok_or_else(|| "Actions commit pull relation is invalid".to_owned())?;
    let matches = pulls
        .iter()
        .filter(|pull| pull.get("number").and_then(Value::as_u64) == Some(locator.pull_request))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(
            "Actions immutable run commit is not related to one requested pull request".into(),
        );
    }
    let pull = object(Some(matches[0]), "Actions pull request relation")?;
    let repository = pull
        .get("base")
        .and_then(Value::as_object)
        .and_then(|base| base.get("repo"))
        .and_then(Value::as_object)
        .and_then(|repo| repo.get("full_name"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Actions pull relation is missing repository identity".to_owned())?;
    if repository != locator.repository {
        return Err("Actions pull relation changes repository identity".into());
    }
    Ok(json!({
        "repository": locator.repository,
        "number": locator.pull_request,
        "url": format!("https://github.com/{}/pull/{}", locator.repository, locator.pull_request)
    }))
}

pub(super) fn select_issue(response: &Value, locator: &Locator) -> Result<Value, String> {
    let events = response
        .as_array()
        .ok_or_else(|| "Actions issue timeline response is invalid".to_owned())?;
    let matches = events
        .iter()
        .filter(|event| {
            let Some(source) = event.get("source").and_then(Value::as_object) else {
                return false;
            };
            let Some(issue) = source.get("issue").and_then(Value::as_object) else {
                return false;
            };
            event.get("event").and_then(Value::as_str) == Some("cross-referenced")
                && source.get("type").and_then(Value::as_str) == Some("issue")
                && issue.get("number").and_then(Value::as_u64) == Some(locator.owning_issue)
                && !issue
                    .get("pull_request")
                    .is_some_and(|pull_request| !pull_request.is_null())
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("Actions pull request timeline does not identify the owning issue".into());
    }
    let event = object(Some(matches[0]), "Actions issue relation")?;
    let source = event
        .get("source")
        .and_then(Value::as_object)
        .ok_or("Actions owning issue relation is missing source identity")?;
    let issue = source
        .get("issue")
        .and_then(Value::as_object)
        .ok_or("Actions owning issue relation is missing issue identity")?;
    let repository = issue
        .get("repository")
        .and_then(Value::as_object)
        .and_then(|repo| repo.get("full_name"))
        .and_then(Value::as_str)
        .ok_or("Actions owning issue relation is missing repository identity")?;
    if repository != locator.repository {
        return Err("Actions owning issue relation changes repository identity".into());
    }
    Ok(json!({
        "repository": locator.repository,
        "number": locator.owning_issue,
        "url": format!("https://github.com/{}/issues/{}", locator.repository, locator.owning_issue),
        "association": "linked-issue-reference"
    }))
}
