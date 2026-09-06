use serde_json::{Map, Value};

mod actions;
mod live;
mod projection;

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

pub(super) fn read_live(locator: &Value, expected_commit: Option<&str>) -> Result<Value, String> {
    let object = locator
        .as_object()
        .ok_or_else(|| "authenticated external finding locator must be an object".to_owned())?;
    let has_graphql = object
        .keys()
        .any(|key| matches!(key.as_str(), "reviewThread" | "reviewComment"));
    let has_actions = object.keys().any(|key| {
        matches!(
            key.as_str(),
            "workflowRun" | "runAttempt" | "job" | "workflowPath" | "jobName" | "stepName"
        )
    });
    match (has_graphql, has_actions) {
        (true, true) => {
            Err("authenticated external finding locator mixes GraphQL and Actions keys".into())
        }
        (false, true) => actions::read_live(locator, expected_commit),
        (true, false) => live::read_graphql_live(locator, expected_commit),
        (false, false) => live::read_graphql_live(locator, expected_commit),
    }
}

pub(super) fn read_actions_live(
    locator: &Value,
    expected_commit: Option<&str>,
) -> Result<Value, String> {
    actions::read_live(locator, expected_commit)
}

pub(super) fn read_live_from_source(
    source: &Value,
    expected_commit: Option<&str>,
) -> Result<Value, String> {
    let capture = source
        .get("capture")
        .and_then(Value::as_object)
        .ok_or_else(|| "authenticated external finding requires capture".to_owned())?;
    match capture.get("method").and_then(Value::as_str) {
        Some("graphql") => live::read_graphql_live_from_source(source, expected_commit),
        Some("actions") => actions::read_live_from_source(source, expected_commit),
        _ => Err("authenticated external finding has an unsupported capture method".into()),
    }
}

pub(super) fn check(
    capture: &Map<String, Value>,
    source: &Map<String, Value>,
) -> Result<(), String> {
    match capture.get("method").and_then(Value::as_str) {
        Some("graphql") => projection::check(capture, source),
        Some("actions") => actions::check(capture, source),
        _ => Err("external finding capture has an unsupported method".into()),
    }
}

pub(super) fn compare_projection(
    expected: &Map<String, Value>,
    actual: &Map<String, Value>,
) -> Result<(), String> {
    match expected
        .get("capture")
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("method"))
        .and_then(Value::as_str)
    {
        Some("graphql") => {
            if RAW_FIELDS
                .iter()
                .any(|field| expected.get(*field) != actual.get(*field))
            {
                return Err("persisted external finding does not match live GitHub source".into());
            }
            Ok(())
        }
        Some("actions") => actions::compare_projection(expected, actual),
        _ => Err("persisted external finding has an unsupported capture method".into()),
    }
}
