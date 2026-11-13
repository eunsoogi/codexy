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
    "explicit_review_requirement",
];

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn classification_request(
    _work_class: &str,
    _low_risk_eligible: bool,
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
        "schema": "codexy.review-selection-request.v2",
        "review_requested": true,
        "risk_assessment": {
            "schema": "codexy.risk-assessment.v1",
            "risk_decisions": strict_triggers,
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
    request["risk_assessment"]["risk_decisions"]
        .as_array_mut()
        .expect("trigger array")
        .retain(|item| item["kind"] != kind);
}

pub(super) fn expected_reviewer() -> Value {
    json!({"name": "codexy-sentinel", "model": "gpt-6.1-sol", "reasoning_effort": "high"})
}
