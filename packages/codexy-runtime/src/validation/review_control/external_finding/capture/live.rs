use std::process::Command;

use serde_json::{Map, Value, json};

use super::super::super::pre_pr::{number, object, reject_unknown, text};
use super::projection::project_response;

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const SCHEMA: &str = "codexy.review-control-external-finding.v1";

#[derive(Debug, Clone)]
pub(super) struct Locator {
    pub(super) repository: String,
    owner: String,
    name: String,
    pub(super) owning_issue: u64,
    pub(super) pull_request: u64,
    pub(super) review_thread: String,
    pub(super) review_comment: String,
}

impl Locator {
    pub(super) fn from_value(value: &Value) -> Result<Self, String> {
        let locator = object(Some(value), "authenticated external finding locator")?;
        reject_unknown(
            locator,
            &[
                "repository",
                "owningIssue",
                "pullRequest",
                "reviewThread",
                "reviewComment",
            ],
            "authenticated external finding locator",
        )?;
        let repository = text(
            locator,
            "repository",
            "authenticated external finding locator",
        )?;
        let mut parts = repository.split('/');
        let owner = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        if owner.is_empty()
            || name.is_empty()
            || parts.next().is_some()
            || repository.chars().any(char::is_whitespace)
        {
            return Err("authenticated external finding locator repository is invalid".into());
        }
        Ok(Self {
            repository: repository.to_owned(),
            owner: owner.to_owned(),
            name: name.to_owned(),
            owning_issue: positive_int(locator, "owningIssue")?,
            pull_request: positive_int(locator, "pullRequest")?,
            review_thread: identifier(locator, "reviewThread")?.to_owned(),
            review_comment: identifier(locator, "reviewComment")?.to_owned(),
        })
    }

    pub(super) fn from_source(source: &Map<String, Value>) -> Result<Self, String> {
        let pull = object(
            source.get("pullRequest"),
            "authenticated external finding pull request",
        )?;
        let issue = object(
            source.get("owningIssue"),
            "authenticated external finding owning issue",
        )?;
        let thread = object(
            source.get("reviewThread"),
            "authenticated external finding review thread",
        )?;
        let comment = object(
            source.get("reviewComment"),
            "authenticated external finding review comment",
        )?;
        let mut locator = Map::new();
        locator.insert(
            "repository".into(),
            Value::String(text(source, "repository", "authenticated external finding")?.to_owned()),
        );
        locator.insert(
            "owningIssue".into(),
            json!(number(
                issue,
                "number",
                "authenticated external finding owning issue"
            )?),
        );
        locator.insert(
            "pullRequest".into(),
            json!(number(
                pull,
                "number",
                "authenticated external finding pull request"
            )?),
        );
        locator.insert(
            "reviewThread".into(),
            Value::String(
                text(thread, "id", "authenticated external finding review thread")?.to_owned(),
            ),
        );
        locator.insert(
            "reviewComment".into(),
            Value::String(
                text(
                    comment,
                    "id",
                    "authenticated external finding review comment",
                )?
                .to_owned(),
            ),
        );
        Self::from_value(&Value::Object(locator))
    }
}

pub(crate) fn read_live(locator: &Value, expected_commit: Option<&str>) -> Result<Value, String> {
    read_locator(Locator::from_value(locator)?, expected_commit)
}

pub(crate) fn read_live_from_source(
    source: &Value,
    expected_commit: Option<&str>,
) -> Result<Value, String> {
    let source = object(Some(source), "authenticated external finding")?;
    read_locator(Locator::from_source(source)?, expected_commit)
}

fn read_locator(locator: Locator, expected_commit: Option<&str>) -> Result<Value, String> {
    let owner = format!("owner={}", locator.owner);
    let name = format!("name={}", locator.name);
    let pull = format!("pullRequest={}", locator.pull_request);
    let issue = format!("owningIssue={}", locator.owning_issue);
    let thread = format!("reviewThread={}", locator.review_thread);
    let comment = format!("reviewComment={}", locator.review_comment);
    let query = format!("query={}", include_str!("query.graphql"));
    let output = Command::new("gh")
        .args([
            "api",
            "graphql",
            "--hostname",
            "github.com",
            "--method",
            "POST",
        ])
        .args(["-f", owner.as_str(), "-f", name.as_str()])
        .args(["-F", pull.as_str(), "-F", issue.as_str()])
        .args([
            "-f",
            thread.as_str(),
            "-f",
            comment.as_str(),
            "-f",
            query.as_str(),
        ])
        .output()
        .map_err(|error| format!("authenticated GitHub external finding read failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "authenticated GitHub external finding read failed: {}",
            bounded_stderr(&output.stderr)
        ));
    }
    if output.stdout.len() > MAX_RESPONSE_BYTES {
        return Err("authenticated GitHub external finding response is too large".into());
    }
    let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        format!("authenticated GitHub external finding response is invalid: {error}")
    })?;
    let projection = project_response(
        object(Some(&response), "authenticated GitHub GraphQL response")?,
        &locator,
        expected_commit,
    )?;
    let raw = json!({"response": response, "projection": projection.clone()});
    let mut source = projection;
    source.insert("schema".into(), Value::String(SCHEMA.into()));
    source.insert(
        "capture".into(),
        json!({"provider":"github", "method":"graphql", "authenticated":true, "raw":raw}),
    );
    Ok(Value::Object(source))
}

fn positive_int(object: &Map<String, Value>, key: &str) -> Result<u64, String> {
    let value = number(object, key, "authenticated external finding locator")?;
    if value == 0 || value > i32::MAX as u64 {
        return Err(format!(
            "authenticated external finding locator {key} must be a positive GraphQL integer"
        ));
    }
    Ok(value)
}

fn identifier<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    let value = text(object, key, "authenticated external finding locator")?;
    if value.chars().any(char::is_whitespace) {
        return Err(format!(
            "authenticated external finding locator {key} is invalid"
        ));
    }
    Ok(value)
}

fn bounded_stderr(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .trim()
        .chars()
        .take(512)
        .collect()
}
