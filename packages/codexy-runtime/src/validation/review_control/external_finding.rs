use std::collections::HashSet;

use serde_json::{Map, Value, json};

use super::pre_pr::{number, object, reject_unknown, text};

mod capture;
mod producer;

pub(super) use producer::{read_actions_live, read_live, refresh_live};

pub(super) const REASON: &str = "authenticated_external_finding_repair";

const SCHEMA: &str = "codexy.review-control-external-finding.v1";
const ISSUE_ASSOCIATIONS: [&str; 3] = [
    "owner-assignment",
    "closing-issue-reference",
    "linked-issue-reference",
];

pub(super) struct Facts {
    pub(super) repository: String,
    pub(super) observed_commit: String,
    pub(super) findings: Vec<Value>,
    pub(super) finding_ids: Vec<String>,
}

pub(super) fn requires_source(control: &Value) -> bool {
    control
        .get("post_cap_re_review")
        .and_then(Value::as_object)
        .and_then(|post_cap| post_cap.get("reason"))
        .and_then(Value::as_str)
        == Some(REASON)
}

pub(super) fn read_from_locators(
    graphql: Option<&Value>,
    actions: Option<&Value>,
    expected_commit: Option<&str>,
) -> Result<Option<Value>, String> {
    match (graphql, actions) {
        (Some(_), Some(_)) => Err(
            "review control producer requires exactly one authenticated external finding locator"
                .into(),
        ),
        (Some(locator), None) => read_live(locator, expected_commit).map(Some),
        (None, Some(locator)) => read_actions_live(locator, expected_commit).map(Some),
        (None, None) => Ok(None),
    }
}

pub(super) fn normalize_producer(control: &mut Value, source: &Value) -> Result<(), String> {
    let facts = check(source)?;
    let control = control
        .as_object_mut()
        .ok_or_else(|| "review control state must be an object".to_owned())?;
    {
        let post_cap = object(control.get("post_cap_re_review"), "post-cap evidence")?;
        if text(post_cap, "reason", "post-cap evidence")? != REASON {
            return Err(
                "authenticated external finding source is only valid for its typed post-cap reason"
                    .into(),
            );
        }
        let change = post_cap
            .get("qualifying_change")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                "external finding repair requires qualifying change evidence".to_owned()
            })?;
        if let Some(existing) = change.get("external_finding") {
            check(existing)?;
            let existing = object(Some(existing), "existing authenticated external finding")?;
            let source = object(Some(source), "authenticated external finding")?;
            if capture::compare_projection(existing, source).is_err() {
                return Err("external finding source changes during producer normalization".into());
            }
        }
        if let Some(existing) = change.get("finding_ids") {
            if string_ids(existing, "qualifying change finding ids")?
                != facts.finding_ids.iter().cloned().collect()
            {
                return Err("qualifying change finding ids do not bind the external source".into());
            }
        }
    }
    let post_cap = control
        .get_mut("post_cap_re_review")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "post-cap evidence must be an object".to_owned())?;
    let change = post_cap
        .get_mut("qualifying_change")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "external finding repair requires qualifying change evidence".to_owned())?;
    change.insert("finding_ids".into(), json!(facts.finding_ids));
    change.insert("external_finding".into(), source.clone());
    Ok(())
}

pub(super) fn check(value: &Value) -> Result<Facts, String> {
    let source = object(Some(value), "authenticated external finding")?;
    reject_unknown(
        source,
        &[
            "schema",
            "capture",
            "repository",
            "owningIssue",
            "pullRequest",
            "source",
            "reviewThread",
            "reviewComment",
            "author",
            "observedCommit",
            "findings",
        ],
        "authenticated external finding",
    )?;
    if text(source, "schema", "authenticated external finding")? != SCHEMA {
        return Err("authenticated external finding has an unsupported schema".into());
    }
    let capture = object(source.get("capture"), "external finding capture")?;
    capture::check(capture, source)?;

    let repository = text(source, "repository", "authenticated external finding")?;
    if repository.split('/').count() != 2
        || repository.split('/').any(str::is_empty)
        || repository.chars().any(char::is_whitespace)
    {
        return Err("external finding repository is invalid".into());
    }
    let owning_issue = object(source.get("owningIssue"), "external finding owning issue")?;
    check_issue(owning_issue, repository)?;
    let pull_request = object(source.get("pullRequest"), "external finding pull request")?;
    reject_unknown(
        pull_request,
        &["repository", "number", "url"],
        "external finding pull request",
    )?;
    if text(pull_request, "repository", "external finding pull request")? != repository {
        return Err("external finding pull request changes repository identity".into());
    }
    let pull_number = number(pull_request, "number", "external finding pull request")?;
    if pull_number == 0
        || text(pull_request, "url", "external finding pull request")?
            != format!("https://github.com/{repository}/pull/{pull_number}")
    {
        return Err("external finding pull request URL is not canonical".into());
    }
    text(source, "author", "authenticated external finding")?;
    let observed_commit = text(source, "observedCommit", "authenticated external finding")?;
    if observed_commit.len() != 40 || !observed_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("external finding observedCommit must be a commit SHA".into());
    }

    let findings = source
        .get("findings")
        .and_then(Value::as_array)
        .filter(|findings| !findings.is_empty())
        .ok_or_else(|| "authenticated external finding must contain findings".to_owned())?;
    let mut ids = HashSet::new();
    let mut normalized = Vec::with_capacity(findings.len());
    for finding in findings {
        let finding = object(Some(finding), "authenticated external finding record")?;
        reject_unknown(
            finding,
            &["id", "path", "test", "exception", "line"],
            "authenticated external finding record",
        )?;
        let id = text(finding, "id", "authenticated external finding record")?;
        if !ids.insert(id.to_owned()) {
            return Err("authenticated external finding ids must be unique".into());
        }
        let path = text(finding, "path", "authenticated external finding record")?;
        if path.starts_with('/')
            || path.contains('\\')
            || path
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err("authenticated external finding path must be repository-relative".into());
        }
        normalized.push(Value::Object(finding.clone()));
    }
    let mut finding_ids = ids.into_iter().collect::<Vec<_>>();
    finding_ids.sort_unstable();
    Ok(Facts {
        repository: repository.to_owned(),
        observed_commit: observed_commit.to_owned(),
        findings: normalized,
        finding_ids,
    })
}

fn check_issue(issue: &Map<String, Value>, repository: &str) -> Result<(), String> {
    reject_unknown(
        issue,
        &["repository", "number", "url", "association"],
        "external finding owning issue",
    )?;
    if text(issue, "repository", "external finding owning issue")? != repository {
        return Err("external finding owning issue changes repository identity".into());
    }
    let number = number(issue, "number", "external finding owning issue")?;
    if number == 0
        || text(issue, "url", "external finding owning issue")?
            != format!("https://github.com/{repository}/issues/{number}")
        || !ISSUE_ASSOCIATIONS.contains(&text(
            issue,
            "association",
            "external finding owning issue",
        )?)
    {
        return Err("external finding owning issue identity is not authenticated".into());
    }
    Ok(())
}

fn string_ids(value: &Value, label: &str) -> Result<HashSet<String>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array"))?;
    let ids = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|id| !id.is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("{label} must contain non-empty strings"))
        })
        .collect::<Result<HashSet<_>, _>>()?;
    if ids.len() != values.len() {
        return Err(format!("{label} must be unique"));
    }
    Ok(ids)
}
