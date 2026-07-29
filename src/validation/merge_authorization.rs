use std::{collections::BTreeSet, fmt, path::Path};

use serde::de::{Deserializer, Error as _, MapAccess, Visitor};
use serde_json::Value;

use super::merge_authorization_contract;

pub(super) fn check(plugin_root: &Path, authorization: &str, pr_state: &str) -> Vec<String> {
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
    check_kind(plugin_root, &authorization, &mut errors);
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

fn check_kind(plugin_root: &Path, authorization: &Value, errors: &mut Vec<String>) {
    match string_field(authorization, "kind") {
        Some("explicit-user-intent") => check_intent(authorization, "user", errors),
        Some("explicit-maintainer-intent") => check_intent(authorization, "maintainer", errors),
        Some("repository-workflow-contract") => merge_authorization_contract::check(plugin_root, authorization, errors),
        _ => errors.push("merge authorization kind must be explicit-user-intent, explicit-maintainer-intent, or repository-workflow-contract".into()),
    }
}

fn check_intent(authorization: &Value, actor: &str, errors: &mut Vec<String>) {
    check_value(authorization, "actor", actor, errors);
    if !string_field(authorization, "sourceReference").is_some_and(|source| {
        source
            .strip_prefix(&format!("{actor}-intent://"))
            .is_some_and(|tail| !tail.trim().is_empty())
    }) {
        errors.push(format!(
            "merge authorization sourceReference must be authenticated {actor} intent"
        ));
    }
    check_value(authorization, "recordIssuer", "maintainer-recorded", errors);
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

fn unique_object(text: &str, label: &str) -> Vec<String> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    deserializer
        .deserialize_map(UniqueObject)
        .err()
        .map(|error| vec![format!("{label} {error}")])
        .unwrap_or_default()
}

struct UniqueObject;

impl<'de> Visitor<'de> for UniqueObject {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object with unique keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(A::Error::custom(format!("must not repeat {key}")));
            }
            let _: Value = map.next_value()?;
        }
        Ok(())
    }
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}
