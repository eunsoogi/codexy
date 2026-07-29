use serde_json::Value;

use super::{merge_authorization_contract, merge_authorization_json::unique_object};

pub(super) fn check(authorization: &str, pr_state: &str) -> Vec<String> {
    let mut errors = unique_object(authorization, "merge authorization");
    errors.extend(unique_object(pr_state, "merge authorization PR state"));
    let authorization = match serde_json::from_str::<Value>(authorization) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("merge authorization JSON error: {error}"));
            return errors;
        }
    };
    let pr_state = match serde_json::from_str::<Value>(pr_state) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("merge authorization PR state JSON error: {error}"));
            return errors;
        }
    };
    check_kind(&authorization, &pr_state, &mut errors);
    check_false(authorization.get("negated"), "negated", &mut errors);
    check_false(authorization.get("revoked"), "revoked", &mut errors);
    check_value(&authorization, "intent", "merge", &mut errors);
    check_value(&authorization, "mergeClass", "squash", &mut errors);
    check_match(&authorization, &pr_state, "prNumber", "number", &mut errors);
    check_match(
        &authorization,
        &pr_state,
        "baseRefName",
        "baseRefName",
        &mut errors,
    );
    check_match(
        &authorization,
        &pr_state,
        "headRefOid",
        "headRefOid",
        &mut errors,
    );
    errors
}

fn check_kind(authorization: &Value, pr_state: &Value, errors: &mut Vec<String>) {
    match string_field(authorization, "kind") {
        Some("explicit-user-intent" | "explicit-maintainer-intent") => check_intent(authorization, pr_state, errors),
        Some("repository-workflow-contract") => {
            merge_authorization_contract::check(authorization, pr_state, errors)
        }
        _ => errors.push("merge authorization kind must be explicit-user-intent, explicit-maintainer-intent, or repository-workflow-contract".into()),
    }
}

fn check_intent(authorization: &Value, pr_state: &Value, errors: &mut Vec<String>) {
    if ["actor", "recordIssuer", "sourceReference"]
        .iter()
        .any(|field| authorization.get(field).is_some())
    {
        errors.push("merge authorization intent must cite an authoritative PR comment, not claimed provenance".into());
    }
    let id = string_field(authorization, "commentId");
    let url = string_field(authorization, "commentUrl");
    let expected = intent_comment(authorization);
    let found = pr_state
        .get("comments")
        .and_then(Value::as_array)
        .map(|comments| {
            comments
                .iter()
                .filter(|comment| {
                    string_field(comment, "id") == id
                        && string_field(comment, "url") == url
                        && authoritative_commenter(comment)
                        && string_field(comment, "body") == expected.as_deref()
                })
                .count()
        });
    let pr_number = typed_value(pr_state, "number").and_then(Value::as_u64);
    let url_prefix = "https://github.com/";
    let url_matches_pr = url.zip(pr_number).is_some_and(|(url, number)| {
        url.starts_with(url_prefix) && url.contains(&format!("/pull/{number}#issuecomment-"))
    });
    if id.is_none_or(str::is_empty) || !url_matches_pr || found != Some(1) {
        errors.push("merge authorization intent must match one OWNER or MEMBER GitHub PR comment with exact squash authorization".into());
    }
}

fn authoritative_commenter(comment: &Value) -> bool {
    comment
        .get("author")
        .and_then(|author| string_field(author, "login"))
        .is_some()
        && string_field(comment, "authorAssociation")
            .is_some_and(|role| matches!(role, "OWNER" | "MEMBER"))
}

fn intent_comment(authorization: &Value) -> Option<String> {
    Some(format!(
        "AUTHORIZE SQUASH MERGE: PR #{} BASE {} HEAD {}",
        typed_value(authorization, "prNumber")?.as_u64()?,
        string_field(authorization, "baseRefName")?,
        string_field(authorization, "headRefOid")?,
    ))
}

fn check_false(value: Option<&Value>, field: &str, errors: &mut Vec<String>) {
    if value.and_then(Value::as_bool) != Some(false) {
        errors.push(format!("merge authorization {field} must be boolean false"));
    }
}

fn check_value(authorization: &Value, field: &str, expected: &str, errors: &mut Vec<String>) {
    if string_field(authorization, field) != Some(expected) {
        errors.push(format!("merge authorization {field} must be {expected:?}"));
    }
}

fn check_match(
    authorization: &Value,
    pr_state: &Value,
    authorization_field: &str,
    pr_state_field: &str,
    errors: &mut Vec<String>,
) {
    let authorization_value = typed_value(authorization, authorization_field);
    let pr_value = typed_value(pr_state, pr_state_field);
    if authorization_value.is_none() || pr_value.is_none() || authorization_value != pr_value {
        errors.push(format!(
            "merge authorization {authorization_field} must match the current PR state"
        ));
    }
}

fn typed_value<'a>(value: &'a Value, field: &str) -> Option<&'a Value> {
    let value = value.get(field)?;
    match field {
        "prNumber" | "number" => value.as_u64().filter(|number| *number > 0).map(|_| value),
        _ => value
            .as_str()
            .filter(|item| !item.trim().is_empty())
            .map(|_| value),
    }
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}
