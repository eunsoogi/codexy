use serde_json::{Value, json};

pub(crate) fn namespace_review_control(state: &mut Value) {
    let object = state.as_object_mut().expect("PR state object");
    let Some(profile) = object.remove("reviewProfile") else {
        return;
    };
    let decision = object.get("reviewDecision").cloned().unwrap_or(Value::Null);
    let mut control = json!({
        "schema": "codexy.review-control-state.v1",
        "profile": profile,
        "decision": decision,
    });
    if let Some(evidence) = object.remove("reviewEvidence") {
        control["evidence"] = evidence;
    }
    if let Some(ledger) = object.remove("reviewLedger") {
        control["ledger"] = ledger;
    }
    object.insert("reviewControl".into(), control);
}

pub(crate) fn namespaced_pr_state(text: &str) -> Result<String, serde_json::Error> {
    let mut state: Value = serde_json::from_str(text)?;
    namespace_review_control(&mut state);
    serde_json::to_string(&state)
}
