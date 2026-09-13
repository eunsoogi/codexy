use serde_json::{Map, Value};

pub(super) const BOOKKEEPING_FIELDS: [&str; 11] = [
    "full_review_count",
    "delta_review_count",
    "terminal_review_count",
    "terminal_review_limit",
    "terminal_review_history",
    "post_cap_re_review",
    "final_disposition",
    "reviewer_migration",
    "pre_pr_import",
    "native_history_recovery",
    "native_history_provenance",
];

const RETIRED_SCHEMA_VALUES: [&str; 12] = [
    "codexy.review-control-external-finding.v1",
    "codexy.review-control-final-disposition.v1",
    "codexy.review-control-final-authority.v1",
    "codexy.review-control-final-authority-comment.v1",
    "codexy.review-control-native-history-request.v1",
    "codexy.review-control-native-history.v1",
    "codexy.review-control-native-history-capture-provenance.v1",
    "codexy.review-control-finding-disposition.v1",
    "codexy.review-control-migration.v1",
    "codexy.review-control-pre-pr-history.v1",
    "codexy.review-control-next-review-eligibility.v1",
    "codexy.review-economics-capture-request.v1",
];

const RETIRED_BOUNDARY_FIELDS: [&str; 12] = [
    "previous_control_state",
    "nativeHistoryRecovery",
    "authenticated_external_finding_locator",
    "authenticated_actions_finding_locator",
    "authenticated_finding_disposition_locator",
    "authenticated_final_disposition_locator",
    "authenticated_external_finding",
    "authenticated_external_finding_capture",
    "authenticated_actions_finding",
    "authenticated_actions_finding_capture",
    "authenticated_finding_disposition",
    "authenticated_finding_disposition_capture",
];

const RETIRED_SOURCE_FIELDS: [&str; 1] = ["finding_disposition"];

/// Reject retired review-control input at the boundary that can admit it.
///
/// This deliberately walks only control aliases and supplied PR-state
/// boundaries. It does not recursively inspect arbitrary capture, review, or
/// historical payloads, which remain immutable records rather than executable
/// review-control input.
pub(super) fn reject_retired_inputs(value: &Value) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };

    inspect_boundary(object, "$", true)?;
    for key in ["control_state", "reviewControl"] {
        if let Some(candidate) = object.get(key) {
            inspect_control(candidate, &format!("$.{key}"))?;
        }
    }
    for key in ["current_pr_state", "previous_pr_state"] {
        if let Some(candidate) = object.get(key) {
            inspect_snapshot(candidate, &format!("$.{key}"))?;
        }
    }
    Ok(())
}

pub(super) fn reject_retired_control(value: &Value) -> Result<(), String> {
    inspect_control(value, "$")
}

pub(super) fn is_compact_control(control: &Map<String, Value>) -> bool {
    !BOOKKEEPING_FIELDS
        .iter()
        .any(|field| control.contains_key(*field))
}

fn inspect_control(value: &Value, path: &str) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    inspect_boundary(object, path, true)?;
    if let Some(field) = ["decision", "evidence", "ledger"]
        .iter()
        .find(|field| object.contains_key(**field))
    {
        return Err(retired_field(path, field));
    }
    Ok(())
}

fn inspect_snapshot(value: &Value, path: &str) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    inspect_boundary(object, path, true)?;
    for key in ["control_state", "reviewControl"] {
        if let Some(candidate) = object.get(key) {
            inspect_control(candidate, &format!("{path}.{key}"))?;
        }
    }
    Ok(())
}

fn inspect_boundary(
    object: &Map<String, Value>,
    path: &str,
    include_source_fields: bool,
) -> Result<(), String> {
    if let Some(field) = BOOKKEEPING_FIELDS
        .iter()
        .find(|field| object.contains_key(**field))
    {
        return Err(retired_field(path, field));
    }
    if let Some(field) = RETIRED_BOUNDARY_FIELDS
        .iter()
        .find(|field| object.contains_key(**field))
    {
        return Err(retired_field(path, field));
    }
    if include_source_fields
        && RETIRED_SOURCE_FIELDS
            .iter()
            .any(|field| object.contains_key(*field))
    {
        return Err(retired_field(path, "finding_disposition"));
    }
    if let Some(schema) = object.get("schema").and_then(Value::as_str)
        && RETIRED_SCHEMA_VALUES.contains(&schema)
    {
        return Err(format!(
            "legacy review-control processing is no longer supported: retired schema at {path}.schema={schema}"
        ));
    }
    Ok(())
}

fn retired_field(path: &str, field: &str) -> String {
    format!(
        "legacy review-control processing is no longer supported: retired input at {path}.{field}"
    )
}
