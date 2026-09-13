use serde_json::json;

use super::policy_support as policy;
use crate::support::TestResult;

#[test]
fn resolver_v2_rejects_ambiguous_or_incomplete_policy_facts() -> TestResult {
    let fixture = policy::fixture()?;
    let invalid = [
        policy::request(vec![]),
        policy::request(vec![
            policy::boundary("same", "production_code", "feature", &[], false, None),
            policy::boundary("same", "runtime_behavior", "feature", &[], false, None),
        ]),
        policy::request(vec![policy::boundary(
            "unknown",
            "unknown_boundary",
            "feature",
            &[],
            false,
            None,
        )]),
        policy::request(vec![json!({
            "id":"missing-facts","kind":"production_code",
            "change":{"purpose":"feature"},"risks":[]
        })]),
        policy::request(vec![policy::boundary(
            "missing-reproduction",
            "production_code",
            "defect_repair",
            &[],
            false,
            None,
        )]),
        policy::request(vec![policy::boundary(
            "unexpected-reproduction",
            "production_code",
            "feature",
            &[],
            false,
            Some(json!({"status":"available"})),
        )]),
        policy::request(vec![policy::boundary(
            "duplicate-risks",
            "production_code",
            "feature",
            &["permission", "permission"],
            false,
            None,
        )]),
        policy::request(vec![policy::boundary(
            "unknown-risk",
            "production_code",
            "feature",
            &["unknown"],
            false,
            None,
        )]),
        policy::request(vec![policy::boundary(
            "high-risk-prose",
            "documentation",
            "feature",
            &["permission"],
            false,
            None,
        )]),
        policy::request(vec![policy::boundary(
            "instruction-first",
            "documentation",
            "instruction_only",
            &[],
            true,
            None,
        )]),
        policy::request(vec![policy::boundary(
            "unavailable-reproduction",
            "production_code",
            "defect_repair",
            &[],
            false,
            Some(json!({"status":"unavailable"})),
        )]),
        json!({"schema":"codexy.tdd-classification-request.v2","boundaries":[{"id":"unexpected","kind":"production_code","change":{"purpose":"feature"},"risks":[],"test_first_required":false,"unexpected":true}]}),
        json!({"schema":"codexy.tdd-classification-request.v3","boundaries":[policy::boundary("unsupported", "production_code", "feature", &[], false, None)]}),
        json!({"schema":"codexy.tdd-classification-request.v2","boundaries":[policy::boundary("blank", "production_code", "feature", &[], false, None)],"boundaries":[]}),
    ];
    for request in invalid {
        policy::assert_rejected(fixture.root(), request, "ambiguous v2 request passed")?;
    }
    policy::assert_text_rejected(
        fixture.root(),
        r#"{"schema":"codexy.tdd-classification-request.v2","boundaries":[],"boundaries":[]}"#,
        "duplicate JSON keys passed",
    )?;
    Ok(())
}
