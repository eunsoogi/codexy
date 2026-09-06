use serde_json::{Map, Value};

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

pub(super) use live::{read_live, read_live_from_source};

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
    if RAW_FIELDS
        .iter()
        .any(|field| expected.get(*field) != actual.get(*field))
    {
        return Err("persisted external finding does not match live GitHub source".into());
    }
    Ok(())
}
