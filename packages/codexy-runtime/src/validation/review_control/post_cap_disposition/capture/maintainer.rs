use std::process::Command;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::super::super::pre_pr::{number, object, text};
use super::{Locator, bounded_response};

#[path = "maintainer_body.rs"]
mod body;

const SCHEMA: &str = "codexy.github-maintainer-policy-decision.v1";

pub(super) fn read(locator: &Locator) -> Result<(Value, Value), String> {
    let owner = format!("owner={}", locator.owner);
    let name = format!("name={}", locator.name);
    let pull = format!("pullRequest={}", locator.pull_request);
    let issue = format!("owningIssue={}", locator.owning_issue);
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
        .args(["-f", query.as_str()])
        .output()
        .map_err(|error| {
            format!("authenticated GitHub maintainer decision read failed: {error}")
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr)
            .trim()
            .chars()
            .take(512)
            .collect::<String>();
        return Err(format!(
            "authenticated GitHub maintainer decision read failed: {}",
            stderr
        ));
    }
    let raw = bounded_response(&output.stdout, "maintainer decision")?;
    let projection = project(&raw, locator)?;
    Ok((raw, projection))
}

pub(super) fn project(raw: &Value, locator: &Locator) -> Result<Value, String> {
    let root = object(Some(raw), "maintainer decision response")?;
    if root
        .get("errors")
        .is_some_and(|errors| errors.as_array().is_none_or(|items| !items.is_empty()))
    {
        return Err("maintainer decision response contains GraphQL errors".into());
    }
    let data = object(root.get("data"), "maintainer decision response data")?;
    let repository = object(data.get("repository"), "maintainer decision repository")?;
    let pull = object(
        repository.get("pullRequest"),
        "maintainer decision pull request",
    )?;
    if number(pull, "number", "maintainer decision pull request")? != locator.pull_request
        || text(pull, "url", "maintainer decision pull request")?
            != format!(
                "https://github.com/{}/pull/{}",
                locator.repository, locator.pull_request
            )
    {
        return Err("maintainer decision response changes pull request identity".into());
    }
    let pull_repo = object(
        pull.get("repository"),
        "maintainer decision pull request repository",
    )?;
    if text(
        pull_repo,
        "nameWithOwner",
        "maintainer decision pull request repository",
    )? != locator.repository
    {
        return Err("maintainer decision response changes repository identity".into());
    }
    let issue = object(repository.get("issue"), "maintainer decision owning issue")?;
    let issue_url = format!(
        "https://github.com/{}/issues/{}",
        locator.repository, locator.owning_issue
    );
    if number(issue, "number", "maintainer decision owning issue")? != locator.owning_issue
        || text(issue, "url", "maintainer decision owning issue")? != issue_url
    {
        return Err("maintainer decision response changes owning issue identity".into());
    }
    let issue_repo = object(
        issue.get("repository"),
        "maintainer decision issue repository",
    )?;
    if text(
        issue_repo,
        "nameWithOwner",
        "maintainer decision issue repository",
    )? != locator.repository
    {
        return Err("maintainer decision response changes owning issue repository".into());
    }
    let comments = object(pull.get("comments"), "maintainer decision comments")?;
    if comments
        .get("pageInfo")
        .and_then(Value::as_object)
        .and_then(|info| info.get("hasNextPage"))
        != Some(&Value::Bool(false))
    {
        return Err("maintainer decision comment lookup is incomplete".into());
    }
    let nodes = comments
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or("maintainer decision comments must list nodes")?;
    let matching = nodes
        .iter()
        .filter(|node| {
            node.get("databaseId").and_then(Value::as_u64) == Some(locator.maintainer_comment)
        })
        .collect::<Vec<_>>();
    let comment = match matching.as_slice() {
        [comment] => *comment,
        [] => {
            return Err(
                "maintainer decision comment is not present in the authenticated lookup".into(),
            );
        }
        _ => return Err("maintainer decision comment lookup contains duplicate identities".into()),
    };
    let comment = object(Some(comment), "maintainer decision comment")?;
    let id = text(comment, "id", "maintainer decision comment")?;
    let url = text(comment, "url", "maintainer decision comment")?;
    if url
        != format!(
            "https://github.com/{}/pull/{}#issuecomment-{}",
            locator.repository, locator.pull_request, locator.maintainer_comment
        )
    {
        return Err("maintainer decision comment URL is not canonical".into());
    }
    let author = object(comment.get("author"), "maintainer decision author")?;
    let author_login = text(author, "login", "maintainer decision author")?;
    let association = text(comment, "authorAssociation", "maintainer decision comment")?;
    if !matches!(association, "OWNER" | "MEMBER") || author_login == "codexy-sentinel" {
        return Err("maintainer decision author is not an independent repository authority".into());
    }
    let created = text(comment, "createdAt", "maintainer decision comment")?;
    let updated = text(comment, "updatedAt", "maintainer decision comment")?;
    if created != updated {
        return Err("maintainer decision comment was edited after creation".into());
    }
    if comment.get("isMinimized") != Some(&Value::Bool(false)) {
        return Err("maintainer decision comment is minimized or lacks minimization state".into());
    }
    let body = text(comment, "body", "maintainer decision comment")?;
    let live_base = text(pull, "baseRefOid", "maintainer decision pull request")?;
    let live_head = text(pull, "headRefOid", "maintainer decision pull request")?;
    let parsed = body::parse(
        body,
        &locator.repository,
        locator.owning_issue,
        locator.pull_request,
        live_base,
        live_head,
    )?;
    let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
    Ok(json!({
        "schema": SCHEMA,
        "repository": locator.repository,
        "owningIssue": {"repository": locator.repository, "number": locator.owning_issue, "url": issue_url},
        "pullRequest": {"repository": locator.repository, "number": locator.pull_request, "url": text(pull, "url", "maintainer decision pull request")?, "baseRefOid": live_base, "headRefOid": live_head},
        "comment": {"id": id, "databaseId": locator.maintainer_comment, "url": url, "author": author_login, "authorAssociation": association, "createdAt": created, "updatedAt": updated, "bodySha256": digest},
        "decision": {"findingId": parsed.finding_id, "path": parsed.finding_path, "reviewer": "codexy-sentinel", "actualModel": parsed.actual_model, "actualReasoningEffort": parsed.actual_reasoning_effort, "accepted": true},
        "sourceProvenance": "orchestrator-transcription"
    }))
}
