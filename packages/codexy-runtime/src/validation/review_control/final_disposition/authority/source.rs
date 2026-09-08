use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::super::super::pre_pr::{number, object, text};
use super::{body, threads};

const COMMENT_SCHEMA: &str = "codexy.review-control-final-authority-comment.v1";

pub(super) fn project(
    raw: &Value,
    locator: &Value,
    expected: &body::Expected<'_>,
) -> Result<Value, String> {
    let root = object(Some(raw), "final disposition authority response")?;
    if root
        .get("errors")
        .is_some_and(|errors| errors.as_array().is_none_or(|items| !items.is_empty()))
    {
        return Err("final disposition authority response contains GraphQL errors".into());
    }
    let data = object(
        root.get("data"),
        "final disposition authority response data",
    )?;
    let repository = object(
        data.get("repository"),
        "final disposition authority repository",
    )?;
    let pull = object(
        repository.get("pullRequest"),
        "final disposition authority pull request",
    )?;
    let pull_number = number(pull, "number", "final disposition authority pull request")?;
    let pull_url = text(pull, "url", "final disposition authority pull request")?;
    let expected_url = format!(
        "https://github.com/{}/pull/{}",
        expected.repository, expected.pull_request
    );
    if pull_number != expected.pull_request
        || pull_url != expected_url
        || text(
            pull,
            "baseRefOid",
            "final disposition authority pull request",
        )? != expected.base
        || text(
            pull,
            "headRefOid",
            "final disposition authority pull request",
        )? != expected.current_head
        || text(pull, "state", "final disposition authority pull request")? != "OPEN"
    {
        return Err("final disposition authority response changes exact PR facts".into());
    }
    let pull_repository = object(
        pull.get("repository"),
        "final disposition authority pull request repository",
    )?;
    if text(
        pull_repository,
        "nameWithOwner",
        "final disposition authority pull request repository",
    )? != expected.repository
    {
        return Err("final disposition authority response changes repository identity".into());
    }
    let issue = object(
        repository.get("issue"),
        "final disposition authority owning issue",
    )?;
    let issue_url = format!(
        "https://github.com/{}/issues/{}",
        expected.repository, expected.owning_issue
    );
    let issue_repository = object(
        issue.get("repository"),
        "final disposition authority owning issue repository",
    )?;
    if number(issue, "number", "final disposition authority owning issue")? != expected.owning_issue
        || text(issue, "url", "final disposition authority owning issue")? != issue_url
        || text(
            issue_repository,
            "nameWithOwner",
            "final disposition authority owning issue repository",
        )? != expected.repository
    {
        return Err("final disposition authority response changes owning issue identity".into());
    }
    if locator.get("repository").and_then(Value::as_str) != Some(expected.repository)
        || locator.get("owningIssue").and_then(Value::as_u64) != Some(expected.owning_issue)
        || locator.get("pullRequest").and_then(Value::as_u64) != Some(expected.pull_request)
    {
        return Err("final disposition authority locator changes target identity".into());
    }
    let threads = threads::project(pull.get("reviewThreads"))?;
    let comments = object(pull.get("comments"), "final disposition authority comments")?;
    threads::check_page(comments, "final disposition authority comments")?;
    let comment_nodes = comments
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or("final disposition authority comments must list nodes")?;
    let comment_id = locator
        .get("maintainerComment")
        .and_then(Value::as_u64)
        .filter(|id| *id > 0)
        .ok_or("final disposition authority locator must contain maintainerComment")?;
    let matching = comment_nodes
        .iter()
        .filter(|comment| comment.get("databaseId") == Some(&json!(comment_id)))
        .collect::<Vec<_>>();
    let comment = match matching.as_slice() {
        [comment] => object(Some(comment), "final disposition authority comment")?,
        [] => return Err("final disposition authority comment is not present".into()),
        _ => return Err("final disposition authority comment identity is duplicated".into()),
    };
    let comment_url = format!(
        "https://github.com/{}/pull/{}#issuecomment-{}",
        expected.repository, expected.pull_request, comment_id
    );
    let author = object(
        comment.get("author"),
        "final disposition authority comment author",
    )?;
    let association = text(
        comment,
        "authorAssociation",
        "final disposition authority comment",
    )?;
    if text(comment, "url", "final disposition authority comment")? != comment_url
        || !matches!(association, "OWNER" | "MEMBER")
        || text(
            author,
            "login",
            "final disposition authority comment author",
        )? == "codexy-sentinel"
        || text(comment, "createdAt", "final disposition authority comment")?
            != text(comment, "updatedAt", "final disposition authority comment")?
        || comment.get("isMinimized") != Some(&Value::Bool(false))
    {
        return Err("final disposition authority comment is not immutable and independent".into());
    }
    let comment_body = text(comment, "body", "final disposition authority comment")?;
    body::check(comment_body, expected)?;
    let body_sha256 = format!("{:x}", Sha256::digest(comment_body.as_bytes()));
    let is_draft = pull
        .get("isDraft")
        .and_then(Value::as_bool)
        .ok_or("final disposition authority pull request must contain isDraft")?;
    let merge_state = text(
        pull,
        "mergeStateStatus",
        "final disposition authority pull request",
    )?;
    Ok(json!({
        "schema": COMMENT_SCHEMA,
        "repository": expected.repository,
        "owningIssue": {"repository": expected.repository, "number": expected.owning_issue, "url": issue_url},
        "pullRequest": {
            "repository": expected.repository,
            "number": expected.pull_request,
            "url": expected_url,
            "baseRefOid": expected.base,
            "headRefOid": expected.current_head
        },
        "prState": {
            "state": "OPEN",
            "isDraft": is_draft,
            "mergeStateStatus": merge_state,
            "reviewThreads": threads
        },
        "comment": {
            "id": text(comment, "id", "final disposition authority comment")?,
            "databaseId": comment_id,
            "url": comment_url,
            "author": text(author, "login", "final disposition authority comment author")?,
            "authorAssociation": association,
            "createdAt": text(comment, "createdAt", "final disposition authority comment")?,
            "updatedAt": text(comment, "updatedAt", "final disposition authority comment")?,
            "bodySha256": body_sha256
        },
        "decision": {
            "kind": "third_block_repair",
            "reviewEventId": expected.review_event_id,
            "sourceHead": expected.source_head,
            "headOid": expected.current_head,
            "baseOid": expected.base,
            "addressedFindingIds": threads::sorted_ids(expected.finding_ids),
            "remainingFindingIds": []
        }
    }))
}
