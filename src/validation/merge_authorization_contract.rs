use serde_json::Value;

pub(super) fn check(record: &Value, pr_state: &Value, errors: &mut Vec<String>) {
    require(record, "target", "current-pull-request", errors);
    let id = string(record, "contractCommentId");
    let url = string(record, "contractCommentUrl");
    let expected = expected_comment(record);
    let found = pr_state
        .get("comments")
        .and_then(Value::as_array)
        .map(|comments| {
            comments
                .iter()
                .filter(|comment| {
                    string(comment, "id") == id
                        && string(comment, "url") == url
                        && comment
                            .get("author")
                            .and_then(|author| string(author, "association"))
                            .is_some_and(|role| matches!(role, "OWNER" | "MEMBER"))
                        && string(comment, "body") == expected.as_deref()
                })
                .count()
        });
    let number = pr_state.get("number").and_then(Value::as_u64);
    let matches_pr = url.zip(number).is_some_and(|(url, number)| {
        url.starts_with("https://github.com/")
            && url.contains(&format!("/pull/{number}#issuecomment-"))
    });
    if id.is_none_or(str::is_empty) || !matches_pr || found != Some(1) {
        errors.push(
            "merge authorization contract must match one OWNER or MEMBER GitHub PR comment".into(),
        );
    }
}

fn require(value: &Value, field: &str, expected: &str, errors: &mut Vec<String>) {
    if value.get(field).and_then(Value::as_str) != Some(expected) {
        errors.push(format!("merge authorization {field} must be {expected:?}"));
    }
}

fn expected_comment(record: &Value) -> Option<String> {
    Some(format!(
        "AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #{} BASE {} HEAD {}",
        record.get("prNumber")?.as_u64()?,
        string(record, "baseRefName")?,
        string(record, "headRefOid")?,
    ))
}

fn string<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}
