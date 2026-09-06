use serde_json::{Map, Value, json};

use super::super::super::pre_pr::{number, object, reject_unknown, text};

mod live;
mod projection;

pub(super) const MAX_LOG_BYTES: usize = 128 * 1024;
const SCHEMA: &str = "codexy.review-control-external-finding.v1";

pub(super) fn read_live(locator: &Value, expected_commit: Option<&str>) -> Result<Value, String> {
    live::read_live(locator, expected_commit)
}

pub(super) fn read_live_from_source(
    source: &Value,
    expected_commit: Option<&str>,
) -> Result<Value, String> {
    live::read_live_from_source(source, expected_commit)
}

pub(super) fn check(
    capture: &Map<String, Value>,
    source: &Map<String, Value>,
) -> Result<(), String> {
    projection::check(capture, source)
}

pub(super) fn compare_projection(
    expected: &Map<String, Value>,
    actual: &Map<String, Value>,
) -> Result<(), String> {
    projection::compare_projection(expected, actual)
}

#[derive(Debug, Clone)]
pub(super) struct Locator {
    pub(super) repository: String,
    pub(super) owning_issue: u64,
    pub(super) pull_request: u64,
    pub(super) workflow_run: u64,
    pub(super) run_attempt: u64,
    pub(super) job: u64,
    pub(super) workflow_path: String,
    pub(super) job_name: String,
    pub(super) step_name: String,
}

impl Locator {
    pub(super) fn from_value(value: &Value) -> Result<Self, String> {
        let object = object(Some(value), "authenticated Actions finding locator")?;
        reject_unknown(
            object,
            &[
                "repository",
                "owningIssue",
                "pullRequest",
                "workflowRun",
                "runAttempt",
                "job",
                "workflowPath",
                "jobName",
                "stepName",
            ],
            "authenticated Actions finding locator",
        )?;
        let repository = text(
            object,
            "repository",
            "authenticated Actions finding locator",
        )?;
        let mut parts = repository.split('/');
        let owner = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        if owner.is_empty()
            || name.is_empty()
            || parts.next().is_some()
            || repository.chars().any(char::is_whitespace)
        {
            return Err("authenticated Actions finding locator repository is invalid".into());
        }
        Ok(Self {
            repository: repository.to_owned(),
            owning_issue: positive(object, "owningIssue")?,
            pull_request: positive(object, "pullRequest")?,
            workflow_run: positive(object, "workflowRun")?,
            run_attempt: positive(object, "runAttempt")?,
            job: positive(object, "job")?,
            workflow_path: nonempty(object, "workflowPath")?.to_owned(),
            job_name: nonempty(object, "jobName")?.to_owned(),
            step_name: nonempty(object, "stepName")?.to_owned(),
        })
    }

    pub(super) fn from_source(source: &Map<String, Value>) -> Result<Self, String> {
        let pull = object(source.get("pullRequest"), "Actions pull request")?;
        let issue = object(source.get("owningIssue"), "Actions owning issue")?;
        let facts = object(source.get("source"), "Actions source facts")?;
        let mut locator = Map::new();
        locator.insert(
            "repository".into(),
            Value::String(text(source, "repository", "Actions source")?.to_owned()),
        );
        locator.insert(
            "owningIssue".into(),
            json!(number(issue, "number", "Actions owning issue")?),
        );
        locator.insert(
            "pullRequest".into(),
            json!(number(pull, "number", "Actions pull request")?),
        );
        for (source_key, locator_key) in [
            ("workflowRun", "workflowRun"),
            ("runAttempt", "runAttempt"),
            ("job", "job"),
            ("workflowPath", "workflowPath"),
            ("jobName", "jobName"),
            ("stepName", "stepName"),
        ] {
            let value = facts
                .get(source_key)
                .ok_or_else(|| format!("Actions source facts are missing {source_key}"))?;
            locator.insert(locator_key.into(), value.clone());
        }
        Self::from_value(&Value::Object(locator))
    }
}

fn positive(object: &Map<String, Value>, key: &str) -> Result<u64, String> {
    let value = number(object, key, "authenticated Actions finding locator")?;
    if value == 0 {
        return Err(format!(
            "authenticated Actions locator {key} must be positive"
        ));
    }
    Ok(value)
}

fn nonempty<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    text(object, key, "authenticated Actions finding locator")
}
