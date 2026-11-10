use serde_json::{Value, json};

pub(super) const MATERIAL_RISK_TRIGGERS: [&str; 11] = [
    "destructive",
    "security",
    "permission",
    "secret",
    "release",
    "high_consequence_external_state",
    "high_risk_guardrail",
    "merge_sensitive",
    "materially_shared_executable_contract",
    "explicit_audit_evidence",
    "explicit_strict_requirement",
];

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn classification_request(
    work_class: &str,
    low_risk_eligible: bool,
    applicable_risks: &[&str],
    collaboration: &[(&str, bool)],
    prior_profile: Option<&str>,
) -> Value {
    let mut strict_triggers = MATERIAL_RISK_TRIGGERS
        .iter()
        .map(|kind| json!({"kind":kind, "applies":applicable_risks.contains(kind)}))
        .collect::<Vec<_>>();
    strict_triggers.extend(
        collaboration
            .iter()
            .map(|(kind, applies)| json!({"kind":kind, "applies":applies})),
    );
    let mut request = json!({
        "schema": "codexy.review-profile-request.v1",
        "classification": {
            "schema": "codexy.workflow-profile-classification.v3",
            "work_class": work_class,
            "low_risk_eligible": low_risk_eligible,
            "strict_triggers": strict_triggers,
        },
    });
    if let Some(prior_profile) = prior_profile {
        request["prior_profile"] = json!(prior_profile);
    }
    request
}

pub(super) fn resolve_profile(request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let plugin_root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    Ok(codexy_runtime::validation::resolve_review_profile(
        &plugin_root,
        &request.to_string(),
    )?)
}

pub(super) fn assert_rejected(request: &Value, expected: &str) {
    let error = resolve_profile(request).expect_err("invalid classification must not resolve");
    assert!(
        error.to_string().contains(expected),
        "unexpected error: {error}"
    );
}

pub(super) fn remove_trigger(request: &mut Value, kind: &str) {
    request["classification"]["strict_triggers"]
        .as_array_mut()
        .expect("trigger array")
        .retain(|item| item["kind"] != kind);
}

pub(super) fn expected_reviewer(profile: &str) -> Value {
    match profile {
        "light" => Value::Null,
        "standard" => json!({
            "name": "codexy-inspector",
            "model": "gpt-6.1-sol",
            "reasoning_effort": "medium",
        }),
        "strict" => json!({
            "name": "codexy-sentinel",
            "model": "gpt-6-astra",
            "reasoning_effort": "xhigh",
        }),
        _ => unreachable!("only the closed profile set is expected"),
    }
}
