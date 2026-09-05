use serde_json::{Map, Value, json};

use super::super::super::pre_pr::{number, object, reject_unknown, text};
use super::live::Locator;

mod identity;
use identity::{
    array, check_comment_identity, check_issue_identity, check_pull_identity, complete, is_oid,
    relative_path,
};

const RAW_FIELDS: [&str; 8] = [
    "repository",
    "owningIssue",
    "pullRequest",
    "reviewThread",
    "reviewComment",
    "author",
    "observedCommit",
    "findings",
];

pub(super) fn check(
    capture: &Map<String, Value>,
    source: &Map<String, Value>,
) -> Result<(), String> {
    check_shape(capture)?;
    let raw = object(capture.get("raw"), "external finding raw capture")?;
    reject_unknown(
        raw,
        &["response", "projection"],
        "external finding raw capture",
    )?;
    let projection = object(raw.get("projection"), "external finding raw projection")?;
    reject_unknown(projection, &RAW_FIELDS, "external finding raw projection")?;
    let response = object(raw.get("response"), "external finding raw response")?;
    if RAW_FIELDS
        .iter()
        .any(|field| source.get(*field) != projection.get(*field))
    {
        return Err("external finding source does not match raw authenticated projection".into());
    }
    let projected = project_response(response, &Locator::from_source(source)?, None)?;
    if RAW_FIELDS
        .iter()
        .any(|field| projection.get(*field) != projected.get(*field))
    {
        return Err("external finding projection does not match raw GitHub response".into());
    }
    Ok(())
}

fn check_shape(capture: &Map<String, Value>) -> Result<(), String> {
    reject_unknown(
        capture,
        &["provider", "method", "authenticated", "raw"],
        "external finding capture",
    )?;
    if text(capture, "provider", "external finding capture")? != "github"
        || text(capture, "method", "external finding capture")? != "graphql"
        || capture.get("authenticated") != Some(&Value::Bool(true))
    {
        return Err("external finding source is not authenticated GitHub GraphQL".into());
    }
    let raw = capture
        .get("raw")
        .ok_or_else(|| "external finding capture requires raw authenticated capture".to_owned())?;
    object(Some(raw), "external finding raw capture")?;
    Ok(())
}

pub(super) fn project_response(
    root: &Map<String, Value>,
    locator: &Locator,
    expected_commit: Option<&str>,
) -> Result<Map<String, Value>, String> {
    if root
        .get("errors")
        .is_some_and(|errors| errors.as_array().is_none_or(|items| !items.is_empty()))
    {
        return Err("authenticated GitHub GraphQL response contains errors".into());
    }
    let data = object(
        root.get("data"),
        "authenticated GitHub GraphQL response data",
    )?;
    let repository = object(
        data.get("repository"),
        "authenticated GitHub GraphQL repository",
    )?;
    let pull = object(
        repository.get("pullRequest"),
        "authenticated GitHub GraphQL pull request",
    )?;
    check_pull_identity(pull, locator, "repository pull request")?;
    let issue = object(
        repository.get("issue"),
        "authenticated GitHub GraphQL owning issue",
    )?;
    check_issue_identity(issue, locator)?;
    let closing = object(
        pull.get("closingIssuesReferences"),
        "authenticated GitHub GraphQL closing issue references",
    )?;
    complete(
        closing,
        "authenticated GitHub GraphQL closing issue references",
    )?;
    let nodes = array(
        closing.get("nodes"),
        "authenticated GitHub GraphQL closing issue references",
    )?;
    if !nodes
        .iter()
        .filter_map(Value::as_object)
        .any(|node| node.get("number").and_then(Value::as_u64) == Some(locator.owning_issue))
    {
        return Err("authenticated GitHub owning issue is not a closing PR reference".into());
    }
    let thread = object(
        data.get("thread"),
        "authenticated GitHub GraphQL review thread",
    )?;
    if text(
        thread,
        "__typename",
        "authenticated GitHub GraphQL review thread",
    )? != "PullRequestReviewThread"
        || text(thread, "id", "authenticated GitHub GraphQL review thread")?
            != locator.review_thread
    {
        return Err("authenticated GitHub review thread does not match locator".into());
    }
    check_pull_identity(
        object(
            thread.get("pullRequest"),
            "authenticated GitHub GraphQL review thread pull request",
        )?,
        locator,
        "review thread pull request",
    )?;
    let path = relative_path(thread, "path", "authenticated GitHub review thread")?;
    let comments = object(
        thread.get("comments"),
        "authenticated GitHub GraphQL review thread comments",
    )?;
    complete(
        comments,
        "authenticated GitHub GraphQL review thread comments",
    )?;
    let comment_nodes = array(
        comments.get("nodes"),
        "authenticated GitHub GraphQL review thread comments",
    )?;
    let thread_comment = comment_nodes
        .iter()
        .filter_map(Value::as_object)
        .find(|comment| {
            comment.get("id").and_then(Value::as_str) == Some(locator.review_comment.as_str())
        })
        .ok_or_else(|| {
            "authenticated GitHub review comment is not in the selected thread".to_owned()
        })?;
    let comment = object(
        data.get("comment"),
        "authenticated GitHub GraphQL review comment",
    )?;
    if text(
        comment,
        "__typename",
        "authenticated GitHub GraphQL review comment",
    )? != "PullRequestReviewComment"
    {
        return Err("authenticated GitHub review comment has an unsupported type".into());
    }
    check_comment_identity(comment, locator, pull)?;
    for field in ["id", "databaseId", "url", "commit", "path"] {
        if comment.get(field) != thread_comment.get(field) {
            return Err("authenticated GitHub review comment readbacks do not match".into());
        }
    }
    if comment.get("path") != Some(&Value::String(path.clone())) {
        return Err("authenticated GitHub review thread and comment paths do not match".into());
    }
    let observed = text(
        object(
            comment.get("commit"),
            "authenticated GitHub review comment commit",
        )?,
        "oid",
        "authenticated GitHub review comment commit",
    )?;
    if !is_oid(observed) {
        return Err("authenticated GitHub review comment commit is not a Git SHA".into());
    }
    if expected_commit.is_some_and(|expected| expected != observed) {
        return Err(
            "authenticated GitHub review comment commit does not match the prior delta head".into(),
        );
    }
    let author = text(
        object(
            comment.get("author"),
            "authenticated GitHub review comment author",
        )?,
        "login",
        "authenticated GitHub review comment author",
    )?;
    let database_id = number(comment, "databaseId", "authenticated GitHub review comment")?;
    if database_id == 0 {
        return Err("authenticated GitHub review comment databaseId is invalid".into());
    }
    let pull_url = text(pull, "url", "authenticated GitHub pull request")?;
    let comment_url = text(comment, "url", "authenticated GitHub review comment")?;
    if comment_url != format!("{pull_url}#discussion_r{database_id}") {
        return Err("authenticated GitHub review comment URL is not canonical".into());
    }
    Ok(Map::from_iter([
        (
            "repository".into(),
            Value::String(locator.repository.clone()),
        ),
        (
            "owningIssue".into(),
            json!({"repository":locator.repository,"number":locator.owning_issue,"url":format!("https://github.com/{}/issues/{}",locator.repository,locator.owning_issue),"association":"closing-issue-reference"}),
        ),
        (
            "pullRequest".into(),
            json!({"repository":locator.repository,"number":locator.pull_request,"url":pull_url}),
        ),
        (
            "reviewThread".into(),
            json!({"id":locator.review_thread,"url":comment_url}),
        ),
        (
            "reviewComment".into(),
            json!({"id":locator.review_comment,"databaseId":database_id,"url":comment_url}),
        ),
        ("author".into(), Value::String(author.to_owned())),
        ("observedCommit".into(), Value::String(observed.to_owned())),
        (
            "findings".into(),
            json!([{"id":format!("github-pr{}-discussion-r{}",locator.pull_request,database_id),"path":path}]),
        ),
    ]))
}
