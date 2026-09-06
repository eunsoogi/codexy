use serde_json::{Map, Value, json};

use super::super::pre_pr::{number, object, reject_unknown, text};

mod ci;
mod maintainer;

const SCHEMA: &str = "codexy.review-control-finding-disposition.v1";
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

pub(super) struct Locator {
    pub(super) repository: String,
    pub(super) owner: String,
    pub(super) name: String,
    pub(super) owning_issue: u64,
    pub(super) pull_request: u64,
    pub(super) maintainer_comment: u64,
}

impl Locator {
    pub(super) fn from_value(value: &Value) -> Result<Self, String> {
        let object = object(Some(value), "authenticated finding disposition locator")?;
        reject_unknown(
            object,
            &[
                "repository",
                "owningIssue",
                "pullRequest",
                "maintainerComment",
            ],
            "authenticated finding disposition locator",
        )?;
        let repository = text(object, "repository", "finding disposition locator")?;
        let mut parts = repository.split('/');
        let owner = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        if owner.is_empty()
            || name.is_empty()
            || parts.next().is_some()
            || repository.chars().any(char::is_whitespace)
        {
            return Err("finding disposition locator repository is invalid".into());
        }
        Ok(Self {
            repository: repository.to_owned(),
            owner: owner.to_owned(),
            name: name.to_owned(),
            owning_issue: positive(object, "owningIssue")?,
            pull_request: positive(object, "pullRequest")?,
            maintainer_comment: positive(object, "maintainerComment")?,
        })
    }

    pub(super) fn value(&self) -> Value {
        json!({
            "repository": self.repository,
            "owningIssue": self.owning_issue,
            "pullRequest": self.pull_request,
            "maintainerComment": self.maintainer_comment
        })
    }
}

pub(super) fn read_live(locator: Locator, expected_head: Option<&str>) -> Result<Value, String> {
    let (ci_raw, ci_projection) = ci::read(&locator)?;
    let (maintainer_raw, maintainer_projection) = maintainer::read(&locator)?;
    if ci_projection["repository"].as_str() != Some(locator.repository.as_str())
        || maintainer_projection["repository"].as_str() != Some(locator.repository.as_str())
        || ci_projection["pullRequest"] != json!(locator.pull_request)
        || maintainer_projection["pullRequest"]["number"] != json!(locator.pull_request)
        || ci_projection["baseRefOid"] != maintainer_projection["pullRequest"]["baseRefOid"]
        || ci_projection["headRefOid"] != maintainer_projection["pullRequest"]["headRefOid"]
    {
        return Err(format!(
            "finding disposition sources disagree on repository, PR, base, or head: ci_repo={:?} md_repo={:?} ci_pr={:?} md_pr={:?} ci_base={:?} md_base={:?} ci_head={:?} md_head={:?}",
            ci_projection["repository"],
            maintainer_projection["repository"],
            ci_projection["pullRequest"],
            maintainer_projection["pullRequest"]["number"],
            ci_projection["baseRefOid"],
            maintainer_projection["pullRequest"]["baseRefOid"],
            ci_projection["headRefOid"],
            maintainer_projection["pullRequest"]["headRefOid"]
        ));
    }
    if expected_head.is_some_and(|head| ci_projection["headRefOid"].as_str() != Some(head)) {
        return Err("finding disposition sources are stale for the current head".into());
    }
    let source = json!({
        "schema": SCHEMA,
        "locator": locator.value(),
        "repository": locator.repository,
        "owningIssue": maintainer_projection["owningIssue"],
        "pullRequest": maintainer_projection["pullRequest"],
        "sources": {
            "currentHeadCi": ci_projection,
            "maintainerDecision": maintainer_projection
        },
        "capture": {
            "provider": "github",
            "method": "graphql+gh-pr-view",
            "authenticated": true,
            "raw": {"currentHeadCi": ci_raw, "maintainerDecision": maintainer_raw},
            "projection": {
                "currentHeadCi": source_projection(&ci_projection),
                "maintainerDecision": source_projection(&maintainer_projection)
            }
        }
    });
    check(source.as_object().ok_or("finding disposition source")?)?;
    Ok(source)
}

pub(super) fn check(source: &Map<String, Value>) -> Result<(), String> {
    let locator = Locator::from_value(
        source
            .get("locator")
            .ok_or_else(|| "finding disposition must retain locator".to_owned())?,
    )?;
    let capture = object(source.get("capture"), "finding disposition capture")?;
    reject_unknown(
        capture,
        &["provider", "method", "authenticated", "raw", "projection"],
        "finding disposition capture",
    )?;
    if text(capture, "provider", "finding disposition capture")? != "github"
        || text(capture, "method", "finding disposition capture")? != "graphql+gh-pr-view"
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err("finding disposition source is not authenticated GitHub".into());
    }
    let raw = object(capture.get("raw"), "finding disposition raw capture")?;
    reject_unknown(
        raw,
        &["currentHeadCi", "maintainerDecision"],
        "finding disposition raw capture",
    )?;
    let projection = object(capture.get("projection"), "finding disposition projection")?;
    let ci_raw = raw
        .get("currentHeadCi")
        .ok_or("finding disposition raw CI")?;
    let md_raw = raw
        .get("maintainerDecision")
        .ok_or("finding disposition raw maintainer")?;
    let ci = ci::project(ci_raw, &locator)?;
    let maintainer = maintainer::project(md_raw, &locator)?;
    let expected_projection = json!({"currentHeadCi": source_projection(&ci), "maintainerDecision": source_projection(&maintainer)});
    if Value::Object(projection.clone()) != expected_projection {
        return Err(
            "finding disposition projection does not match raw authenticated sources".into(),
        );
    }
    let sources = object(source.get("sources"), "finding disposition sources")?;
    reject_unknown(
        sources,
        &["currentHeadCi", "maintainerDecision"],
        "finding disposition sources",
    )?;
    if sources.get("currentHeadCi") != Some(&ci)
        || sources.get("maintainerDecision") != Some(&maintainer)
    {
        return Err("finding disposition sources do not match raw authenticated projection".into());
    }
    if source.get("repository") != Some(&Value::String(locator.repository.clone()))
        || source.get("owningIssue") != maintainer.get("owningIssue")
        || source.get("pullRequest") != maintainer.get("pullRequest")
    {
        return Err("finding disposition identity does not match authenticated sources".into());
    }
    Ok(())
}

fn source_projection(value: &Value) -> Value {
    value.clone()
}

fn positive(object: &Map<String, Value>, key: &str) -> Result<u64, String> {
    let value = number(object, key, "finding disposition locator")?;
    if value == 0 {
        return Err(format!(
            "finding disposition locator {key} must be positive"
        ));
    }
    Ok(value)
}

pub(super) fn bounded_response(output: &[u8], label: &str) -> Result<Value, String> {
    if output.len() > MAX_RESPONSE_BYTES {
        return Err(format!(
            "authenticated GitHub {label} response is too large"
        ));
    }
    serde_json::from_slice(output)
        .map_err(|error| format!("authenticated GitHub {label} response is invalid: {error}"))
}
