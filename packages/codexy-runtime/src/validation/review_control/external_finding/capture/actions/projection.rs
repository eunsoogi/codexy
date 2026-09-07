use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::super::super::super::pre_pr::{number, object, reject_unknown, text};
use super::Locator;

mod parser;
use parser::parse_log;
mod ownership;
mod shape;
use shape::check_shape;

const FIELDS: [&str; 7] = [
    "repository",
    "owningIssue",
    "pullRequest",
    "source",
    "author",
    "observedCommit",
    "findings",
];

pub(super) fn normalize_log(log: &str) -> String {
    parser::normalize_log(log)
}

pub(super) fn scoped_log(
    log: &str,
    step: &Map<String, Value>,
    repository: &str,
    ambiguous_step_boundary: bool,
) -> Result<String, String> {
    parser::scoped_log(log, step, repository, ambiguous_step_boundary)
}

pub(super) fn check(
    capture: &Map<String, Value>,
    source: &Map<String, Value>,
) -> Result<(), String> {
    check_shape(capture)?;
    reject_unknown(
        source,
        &[
            "schema",
            "capture",
            "repository",
            "owningIssue",
            "pullRequest",
            "source",
            "author",
            "observedCommit",
            "findings",
        ],
        "Actions external finding",
    )?;
    let facts = object(source.get("source"), "Actions source facts")?;
    reject_unknown(
        facts,
        &[
            "kind",
            "event",
            "workflowRun",
            "runAttempt",
            "workflowId",
            "workflowPath",
            "job",
            "jobName",
            "jobAttempt",
            "jobHeadSha",
            "stepNumber",
            "stepName",
            "stepConclusion",
        ],
        "Actions source facts",
    )?;
    let raw = object(capture.get("raw"), "Actions raw capture")?;
    let projection = object(raw.get("projection"), "Actions raw projection")?;
    reject_unknown(projection, &FIELDS, "Actions raw projection")?;
    let locator = Locator::from_source(source)?;
    let expected = build(raw, &locator, None)?;
    if FIELDS.iter().any(|field| {
        source.get(*field) != projection.get(*field) || source.get(*field) != expected.get(*field)
    }) {
        return Err("Actions source does not match its authenticated projection".into());
    }
    Ok(())
}

pub(super) fn compare_projection(
    expected: &Map<String, Value>,
    actual: &Map<String, Value>,
) -> Result<(), String> {
    if FIELDS
        .iter()
        .any(|field| expected.get(*field) != actual.get(*field))
    {
        return Err("persisted external finding does not match live GitHub Actions source".into());
    }
    Ok(())
}

pub(super) fn project(
    raw: &Map<String, Value>,
    locator: &Locator,
    expected_commit: Option<&str>,
) -> Result<Map<String, Value>, String> {
    build(raw, locator, expected_commit)
}

fn build(
    raw: &Map<String, Value>,
    locator: &Locator,
    expected_commit: Option<&str>,
) -> Result<Map<String, Value>, String> {
    let repository = text(raw, "repository", "Actions raw capture")?;
    let run = object(raw.get("run"), "Actions workflow run")?;
    let run_id = positive(run, "id", "Actions workflow run")?;
    let attempt = positive(run, "run_attempt", "Actions workflow run")?;
    let workflow_id = positive(run, "workflow_id", "Actions workflow run")?;
    let workflow_path = text(run, "path", "Actions workflow run")?;
    let event = text(run, "event", "Actions workflow run")?;
    let observed = text(run, "head_sha", "Actions workflow run")?;
    if event != "pull_request"
        || text(run, "status", "Actions workflow run")? != "completed"
        || text(run, "conclusion", "Actions workflow run")? != "failure"
        || !is_oid(observed)
    {
        return Err("Actions workflow run is not a completed failure on a pull request".into());
    }
    if expected_commit.is_some_and(|expected| expected != observed) {
        return Err("Actions workflow run head does not match the prior delta head".into());
    }
    let job = object(raw.get("job"), "Actions job")?;
    let job_id = positive(job, "id", "Actions job")?;
    let job_attempt = positive(job, "run_attempt", "Actions job")?;
    if job_attempt != attempt
        || text(job, "head_sha", "Actions job")? != observed
        || text(job, "status", "Actions job")? != "completed"
        || text(job, "conclusion", "Actions job")? != "failure"
    {
        return Err("Actions job is not bound to the exact failed run attempt".into());
    }
    let step = object(raw.get("step"), "Actions failed step")?;
    let step_number = positive(step, "number", "Actions failed step")?;
    if text(step, "status", "Actions failed step")? != "completed"
        || text(step, "conclusion", "Actions failed step")? != "failure"
    {
        return Err("Actions failed step is not terminal".into());
    }
    let relation = object(raw.get("relation"), "Actions pull request relation")?;
    let pull_number = positive(relation, "number", "Actions pull request relation")?;
    if text(relation, "repository", "Actions pull request relation")? != repository
        || text(relation, "url", "Actions pull request relation")?
            != format!("https://github.com/{repository}/pull/{pull_number}")
    {
        return Err("Actions pull request relation changes repository identity".into());
    }
    let issue = object(raw.get("issueRelation"), "Actions issue relation")?;
    let issue_number = positive(issue, "number", "Actions issue relation")?;
    if text(issue, "repository", "Actions issue relation")? != repository
        || text(issue, "url", "Actions issue relation")?
            != format!("https://github.com/{repository}/issues/{issue_number}")
        || !matches!(
            text(issue, "association", "Actions issue relation")?,
            "owner-assignment" | "closing-issue-reference" | "linked-issue-reference"
        )
    {
        return Err("Actions issue relation is not authenticated".into());
    }
    if locator.repository != repository
        || locator.owning_issue != issue_number
        || locator.pull_request != pull_number
        || locator.workflow_run != run_id
        || locator.run_attempt != attempt
        || locator.job != job_id
        || locator.workflow_path != workflow_path
        || locator.job_name != text(job, "name", "Actions job")?
        || locator.step_name != text(step, "name", "Actions failed step")?
    {
        return Err("authenticated GitHub Actions response does not match locator".into());
    }
    let source_ownership =
        ownership::authenticated_projection(raw.get("sourceOwnership"), locator)?;
    let source_pull = object(
        source_ownership.get("pullRequest"),
        "Actions source ownership pull request projection",
    )?;
    let source_issue = object(
        source_ownership.get("owningIssue"),
        "Actions source ownership issue projection",
    )?;
    if source_pull.get("number") != Some(&Value::from(pull_number))
        || source_issue.get("number") != Some(&Value::from(issue_number))
        || source_pull.get("repository") != Some(&Value::String(repository.to_owned()))
        || source_issue.get("repository") != Some(&Value::String(repository.to_owned()))
    {
        return Err(
            "Actions timeline relation does not match authenticated source ownership".into(),
        );
    }
    let failure = parse_log(text(raw, "log", "Actions raw capture")?, repository)?;
    let finding_id = finding_id(run_id, attempt, job_id, step_number, &failure.test);
    Ok(Map::from_iter([
        ("repository".into(), Value::String(repository.to_owned())),
        ("owningIssue".into(), Value::Object(source_issue.clone())),
        ("pullRequest".into(), Value::Object(source_pull.clone())),
        (
            "source".into(),
            json!({"kind":"github-actions","event":event,"workflowRun":run_id,"runAttempt":attempt,"workflowId":workflow_id,"workflowPath":workflow_path,"job":job_id,"jobName":job["name"],"jobAttempt":job_attempt,"jobHeadSha":job["head_sha"],"stepNumber":step_number,"stepName":step["name"],"stepConclusion":step["conclusion"]}),
        ),
        ("author".into(), Value::String("github-actions".into())),
        ("observedCommit".into(), Value::String(observed.to_owned())),
        (
            "findings".into(),
            json!([{"id":finding_id,"path":failure.path,"test":failure.test,"exception":failure.exception,"line":failure.line}]),
        ),
    ]))
}

pub(super) fn project_source_ownership(
    response: &Value,
    locator: &Locator,
) -> Result<Map<String, Value>, String> {
    ownership::project(response, locator)
}

fn finding_id(run: u64, attempt: u64, job: u64, step: u64, test: &str) -> String {
    let digest = Sha256::digest(format!("{run}:{attempt}:{job}:{step}:{test}").as_bytes());
    let suffix = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("github-actions-{run}-{attempt}-{job}-{step}-{suffix}")
}

fn positive(object: &Map<String, Value>, key: &str, label: &str) -> Result<u64, String> {
    let value = number(object, key, label)?;
    if value == 0 {
        return Err(format!("{label} {key} must be positive"));
    }
    Ok(value)
}

fn is_oid(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
