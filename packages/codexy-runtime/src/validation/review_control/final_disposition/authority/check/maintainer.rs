use std::collections::HashSet;

use serde_json::{Map, Value, json};

use super::super::super::super::pre_pr::object;
use super::super::super::fields::{reject_unknown, text};
use crate::validation::review_thread_evidence;

const COMMENT_SCHEMA: &str = "codexy.review-control-final-authority-comment.v1";

#[allow(clippy::too_many_arguments)]
pub(crate) fn check_maintainer(
    value: &Value,
    authority: &Map<String, Value>,
    source_head: &str,
    current_head: &str,
    base: &str,
    event_id: &str,
    issue_number: u64,
    pull_number: u64,
    finding_ids: &HashSet<String>,
) -> Result<(), String> {
    let source = object(Some(value), "final disposition maintainer decision")?;
    reject_unknown(
        source,
        &[
            "schema",
            "repository",
            "owningIssue",
            "pullRequest",
            "prState",
            "comment",
            "decision",
        ],
        "final disposition maintainer decision",
    )?;
    let repository = text(authority, "repository", "final disposition authority")?;
    if text(source, "schema", "final disposition maintainer decision")? != COMMENT_SCHEMA
        || text(
            source,
            "repository",
            "final disposition maintainer decision",
        )? != repository
    {
        return Err("final disposition maintainer decision changes identity".into());
    }
    check_identity(
        source,
        repository,
        issue_number,
        pull_number,
        base,
        current_head,
    )?;
    let decision = object(source.get("decision"), "final disposition decision")?;
    reject_unknown(
        decision,
        &[
            "kind",
            "reviewEventId",
            "sourceHead",
            "headOid",
            "baseOid",
            "addressedFindingIds",
            "remainingFindingIds",
        ],
        "final disposition decision",
    )?;
    if text(decision, "kind", "final disposition decision")? != "third_block_repair"
        || text(decision, "reviewEventId", "final disposition decision")? != event_id
        || text(decision, "sourceHead", "final disposition decision")? != source_head
        || text(decision, "headOid", "final disposition decision")? != current_head
        || text(decision, "baseOid", "final disposition decision")? != base
        || decision.get("remainingFindingIds") != Some(&json!([]))
        || ids(decision.get("addressedFindingIds"))? != *finding_ids
    {
        return Err("final disposition maintainer decision does not bind exact findings".into());
    }
    let state = source
        .get("prState")
        .ok_or("final disposition authority must retain PR state")?;
    check_pr_state(state)?;
    check_comment(source.get("comment"), authority)?;
    Ok(())
}

fn check_identity(
    source: &Map<String, Value>,
    repository: &str,
    issue_number: u64,
    pull_number: u64,
    base: &str,
    current_head: &str,
) -> Result<(), String> {
    let issue = object(source.get("owningIssue"), "final disposition owning issue")?;
    reject_unknown(
        issue,
        &["repository", "number", "url"],
        "final disposition owning issue",
    )?;
    if text(issue, "repository", "final disposition owning issue")? != repository
        || issue.get("number") != Some(&json!(issue_number))
        || text(issue, "url", "final disposition owning issue")?
            != format!("https://github.com/{repository}/issues/{issue_number}")
    {
        return Err("final disposition maintainer decision changes owning issue identity".into());
    }
    let pull = object(source.get("pullRequest"), "final disposition pull request")?;
    reject_unknown(
        pull,
        &["repository", "number", "url", "baseRefOid", "headRefOid"],
        "final disposition pull request",
    )?;
    if text(pull, "repository", "final disposition pull request")? != repository
        || pull.get("number") != Some(&json!(pull_number))
        || text(pull, "url", "final disposition pull request")?
            != format!("https://github.com/{repository}/pull/{pull_number}")
        || text(pull, "baseRefOid", "final disposition pull request")? != base
        || text(pull, "headRefOid", "final disposition pull request")? != current_head
    {
        return Err("final disposition maintainer decision changes pull request identity".into());
    }
    Ok(())
}

fn check_pr_state(value: &Value) -> Result<(), String> {
    let state = object(Some(value), "final disposition authority PR state")?;
    if text(state, "state", "final disposition authority PR state")? != "OPEN"
        || state.get("isDraft") != Some(&Value::Bool(false))
        || text(
            state,
            "mergeStateStatus",
            "final disposition authority PR state",
        )? != "CLEAN"
    {
        return Err("final disposition authority PR state is not open, ready, and clean".into());
    }
    let threads = state
        .get("reviewThreads")
        .ok_or("final disposition authority PR state must retain reviewThreads")?;
    if let Some(error) = review_thread_evidence::check(threads) {
        return Err(error);
    }
    let nodes = threads
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or("final disposition authority PR state must list review thread nodes")?;
    if nodes
        .iter()
        .any(|thread| thread.get("isResolved") != Some(&Value::Bool(true)))
    {
        return Err("final disposition authority PR state has unresolved review threads".into());
    }
    Ok(())
}

fn check_comment(value: Option<&Value>, authority: &Map<String, Value>) -> Result<(), String> {
    let comment = object(value, "final disposition authority comment")?;
    let repository = text(authority, "repository", "final disposition authority")?;
    let pull = authority
        .get("pullRequest")
        .and_then(Value::as_object)
        .ok_or("final disposition authority must retain pull request")?;
    let number = pull
        .get("number")
        .and_then(Value::as_u64)
        .ok_or("final disposition authority pull request must contain number")?;
    let database_id = comment
        .get("databaseId")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or("final disposition authority comment must contain positive databaseId")?;
    let author = text(comment, "author", "final disposition authority comment")?;
    let association = text(
        comment,
        "authorAssociation",
        "final disposition authority comment",
    )?;
    let created = text(comment, "createdAt", "final disposition authority comment")?;
    let updated = text(comment, "updatedAt", "final disposition authority comment")?;
    reject_unknown(
        comment,
        &[
            "id",
            "databaseId",
            "url",
            "author",
            "authorAssociation",
            "createdAt",
            "updatedAt",
            "bodySha256",
        ],
        "final disposition authority comment",
    )?;
    text(comment, "id", "final disposition authority comment")?;
    if text(comment, "url", "final disposition authority comment")?
        != format!(
            "https://github.com/{repository}/pull/{}#issuecomment-{database_id}",
            number
        )
        || !matches!(association, "OWNER" | "MEMBER")
        || author == "codexy-sentinel"
        || created != updated
        || !is_sha256(text(
            comment,
            "bodySha256",
            "final disposition authority comment",
        )?)
    {
        return Err("final disposition authority comment metadata is invalid".into());
    }
    Ok(())
}

fn ids(value: Option<&Value>) -> Result<HashSet<String>, String> {
    let values = value
        .and_then(Value::as_array)
        .ok_or("final disposition decision must list addressedFindingIds")?;
    let mut ids = HashSet::new();
    for value in values {
        let id = value
            .as_str()
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .ok_or("final disposition decision finding ids must be strings")?;
        if !ids.insert(id) {
            return Err("final disposition decision finding ids must be unique".into());
        }
    }
    Ok(ids)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
