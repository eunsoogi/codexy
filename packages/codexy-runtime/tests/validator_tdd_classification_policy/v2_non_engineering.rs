use serde_json::json;

use super::policy_support as policy;
use crate::support::TestResult;

const BEHAVIOR: &[&str] = &["requirement_linked_behavioral_verification"];
const PROPORTIONAL: &[&str] = &["proportional_structural_or_behavioral_proof"];

#[test]
fn documentation_defect_keeps_proportional_proof_without_red() -> TestResult {
    let fixture = policy::fixture()?;
    let request = policy::request(vec![policy::boundary(
        "docs-defect",
        "documentation",
        "defect_repair",
        &[],
        false,
        Some(json!({"status":"available"})),
    )]);
    let expected = policy::expected(
        "non_engineering",
        false,
        "not_applicable",
        &[],
        &["documentation"],
        vec![policy::obligation(
            "docs-defect",
            "documentation",
            false,
            "not_applicable",
            &[],
            PROPORTIONAL,
        )],
    );
    policy::assert_v2(fixture.root(), request, expected)
}

#[test]
fn mixed_engineering_and_documentation_defects_keep_separate_duties() -> TestResult {
    let fixture = policy::fixture()?;
    let request = policy::request(vec![
        policy::boundary(
            "code-defect",
            "production_code",
            "defect_repair",
            &[],
            false,
            Some(json!({"status":"available"})),
        ),
        policy::boundary(
            "docs-defect",
            "documentation",
            "defect_repair",
            &[],
            false,
            Some(json!({"status":"available"})),
        ),
    ]);
    let expected = policy::expected(
        "mixed",
        true,
        "required",
        &["production_code"],
        &["documentation"],
        vec![
            policy::obligation(
                "code-defect",
                "production_code",
                true,
                "required",
                &["faithful_red_before_fix"],
                BEHAVIOR,
            ),
            policy::obligation(
                "docs-defect",
                "documentation",
                false,
                "not_applicable",
                &[],
                PROPORTIONAL,
            ),
        ],
    );
    policy::assert_v2(fixture.root(), request, expected)
}
