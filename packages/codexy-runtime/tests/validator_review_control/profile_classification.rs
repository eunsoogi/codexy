use std::{fs, path::Path};

use serde_json::{Value, json};

use crate::support::{self, TestResult};

use super::resolve_profile;

const STRICT_TRIGGERS: [&str; 11] = [
    "destructive",
    "security",
    "permission",
    "secret",
    "release",
    "high_consequence_external_state",
    "high_risk_guardrail",
    "merge_sensitive",
    "durable_delegation",
    "multi_lane_ownership",
    "explicit_audit_evidence",
];
const POLICY: &str = "skills/orchestration/references/review-profiles.json";

#[test]
fn review_profile_is_derived_from_exhaustive_typed_classification() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    assert_profile(fixture.root(), classified("low_risk", true, None), "light")?;
    assert_profile(fixture.root(), classified("middle", false, None), "standard")?;
    for trigger in STRICT_TRIGGERS {
        assert_profile(fixture.root(), classified("low_risk", true, Some(trigger)), "strict")?;
    }
    Ok(())
}

#[test]
fn classification_rejects_omission_partial_unknown_and_downgrade_inputs() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    for request in [
        json!({"schema":"codexy.review-profile-request.v1","classification":{
            "schema":"codexy.workflow-profile-classification.v2","work_class":"low_risk","low_risk_eligible":true,"strict_triggers":[]
        }}),
        json!({"schema":"codexy.review-profile-request.v1","classification":{
            "schema":"codexy.workflow-profile-classification.v2","work_class":"low_risk","low_risk_eligible":true,
            "strict_triggers":[{"kind":"security","applies":false}]
        }}),
        json!({"schema":"codexy.review-profile-request.v1","classification":{
            "schema":"codexy.workflow-profile-classification.v2","work_class":"low_risk","low_risk_eligible":true,
            "strict_triggers":[{"kind":"unknown","applies":false}]
        }}),
        json!({"schema":"codexy.review-profile-request.v1","classification":{
            "schema":"codexy.workflow-profile-classification.v2","work_class":"low_risk","low_risk_eligible":false,
            "strict_triggers":trigger_decisions(None)
        }}),
        json!({"schema":"codexy.review-profile-request.v1","classification":{
            "schema":"codexy.workflow-profile-classification.v2","work_class":"middle","low_risk_eligible":true,
            "strict_triggers":trigger_decisions(None)
        }}),
        json!({"schema":"codexy.review-profile-request.v1","classification":{
            "schema":"codexy.workflow-profile-classification.v2","work_class":"low_risk","low_risk_eligible":"yes",
            "strict_triggers":trigger_decisions(Some("security"))
        }}),
    ] {
        assert!(
            !resolve_profile(fixture.root(), request)?.status.success(),
            "incomplete or contradictory classification must fail closed"
        );
    }
    Ok(())
}

#[test]
fn classification_validates_the_base_record_before_strict_escalation() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let mut unknown_class = classified("unknown", false, Some("security"));
    let mut duplicate_trigger = classified("middle", false, Some("release"));
    duplicate_trigger["classification"]["strict_triggers"]
        .as_array_mut()
        .expect("trigger decisions")
        .push(json!({"kind":"release","applies":true}));
    for request in [&mut unknown_class, &mut duplicate_trigger] {
        assert!(
            !resolve_profile(fixture.root(), request.take())?.status.success(),
            "an invalid classification must fail before strict escalation"
        );
    }
    Ok(())
}

#[test]
fn policy_requires_the_declared_issue_terminal_review_limit() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(POLICY)])?;
    assert_eq!(
        support::fixture_mutable_files(fixture.root()),
        Some(vec![Path::new(POLICY).to_path_buf()])
    );
    let policy_path = fixture.root().join(POLICY);
    let policy: Value = serde_json::from_slice(&fs::read(&policy_path)?)?;

    let mut accepted = policy.clone();
    accepted["issue_terminal_review_limit"] = json!(3);
    fs::write(&policy_path, serde_json::to_vec(&accepted)?)?;
    assert!(resolve_profile(fixture.root(), classified("middle", false, None))?.status.success());

    for value in [json!(0), json!(4), json!("3")] {
        let mut invalid = policy.clone();
        invalid["issue_terminal_review_limit"] = value;
        fs::write(&policy_path, serde_json::to_vec(&invalid)?)?;
        assert!(!resolve_profile(fixture.root(), classified("middle", false, None))?.status.success());
    }
    Ok(())
}

fn assert_profile(root: &std::path::Path, request: Value, expected: &str) -> TestResult {
    let output = resolve_profile(root, request)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?["profile"], expected);
    Ok(())
}

fn classified(work_class: &str, low_risk_eligible: bool, applies: Option<&str>) -> Value {
    json!({
        "schema":"codexy.review-profile-request.v1",
        "classification": {
            "schema":"codexy.workflow-profile-classification.v2",
            "work_class":work_class,
            "low_risk_eligible":low_risk_eligible,
            "strict_triggers":trigger_decisions(applies)
        }
    })
}

fn trigger_decisions(applies: Option<&str>) -> Vec<Value> {
    STRICT_TRIGGERS
        .iter()
        .map(|kind| json!({"kind":kind,"applies":Some(*kind) == applies}))
        .collect()
}
