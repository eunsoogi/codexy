use serde_json::Value;

pub(super) fn predecessor_has_pre_pr_history(state: Option<&Value>) -> bool {
    state
        .and_then(Value::as_object)
        .and_then(|state| state.get("reviewControl"))
        .and_then(Value::as_object)
        .is_some_and(|control| control.contains_key("pre_pr_import"))
}

pub(super) fn qualifying_change_from_head(control: &Value) -> Option<&str> {
    qualifying_change(control, "from_head")
}

pub(super) fn qualifying_change_to_head(control: &Value) -> Option<&str> {
    qualifying_change(control, "to_head")
}

pub(super) fn has_caller_supplied_finding(request: &Value) -> bool {
    [
        "authenticated_external_finding",
        "authenticated_external_finding_capture",
        "authenticated_actions_finding",
        "authenticated_actions_finding_capture",
        "authenticated_finding_disposition",
        "authenticated_finding_disposition_capture",
        "finding_disposition",
    ]
    .into_iter()
    .any(|key| request.get(key).is_some())
}

fn qualifying_change<'a>(control: &'a Value, key: &str) -> Option<&'a str> {
    control
        .get("post_cap_re_review")
        .and_then(Value::as_object)
        .and_then(|post_cap| post_cap.get("qualifying_change"))
        .and_then(Value::as_object)
        .and_then(|change| change.get(key))
        .and_then(Value::as_str)
        .filter(|head| !head.is_empty())
}
