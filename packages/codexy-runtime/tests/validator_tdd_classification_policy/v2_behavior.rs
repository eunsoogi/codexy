use serde_json::json;

use super::policy_support as policy;
use super::policy_support::BEHAVIOR;
use crate::support::TestResult;

#[test]
fn resolver_v2_separates_behavioral_tests_from_test_first_sequencing() -> TestResult {
    let fixture = policy::fixture()?;
    let cases = [
        case(
            policy::boundary("feature", "production_code", "feature", &[], false, None),
            "engineering",
            true,
            "optional",
            &["production_code"],
            &[],
            policy::obligation(
                "feature",
                "production_code",
                true,
                "optional",
                &[],
                BEHAVIOR,
            ),
        ),
        case(
            policy::boundary(
                "defect",
                "production_code",
                "defect_repair",
                &[],
                false,
                Some(json!({"status":"available"})),
            ),
            "engineering",
            true,
            "required",
            &["production_code"],
            &[],
            policy::obligation(
                "defect",
                "production_code",
                true,
                "required",
                &["faithful_red_before_fix"],
                BEHAVIOR,
            ),
        ),
        case(
            policy::boundary(
                "unavailable-defect",
                "production_code",
                "defect_repair",
                &[],
                false,
                Some(
                    json!({"status":"unavailable","reason":"no stable reproduction environment","alternative":"run focused invariant and regression checks before the repair"}),
                ),
            ),
            "engineering",
            true,
            "optional",
            &["production_code"],
            &[],
            policy::obligation(
                "unavailable-defect",
                "production_code",
                true,
                "optional",
                &["justified_alternative_before_change"],
                BEHAVIOR,
            ),
        ),
        case(
            policy::boundary(
                "refactor",
                "production_code",
                "behavior_preserving_refactor",
                &[],
                false,
                None,
            ),
            "engineering",
            true,
            "optional",
            &["production_code"],
            &[],
            policy::obligation(
                "refactor",
                "production_code",
                true,
                "optional",
                &["green_or_characterization_baseline"],
                BEHAVIOR,
            ),
        ),
        case(
            policy::boundary(
                "permission",
                "runtime_behavior",
                "feature",
                &["permission"],
                false,
                None,
            ),
            "engineering",
            true,
            "optional",
            &["runtime_behavior"],
            &[],
            policy::obligation(
                "permission",
                "runtime_behavior",
                true,
                "optional",
                &["permission_invariants_before_change"],
                BEHAVIOR,
            ),
        ),
        case(
            policy::boundary(
                "guidance",
                "instruction_only_skill",
                "instruction_only",
                &[],
                false,
                None,
            ),
            "non_engineering",
            false,
            "not_applicable",
            &[],
            &["instruction_only_skill"],
            policy::obligation(
                "guidance",
                "instruction_only_skill",
                false,
                "not_applicable",
                &[],
                PROPORTIONAL,
            ),
        ),
        (
            policy::request(vec![
                policy::boundary("code", "production_code", "feature", &[], false, None),
                policy::boundary("docs", "documentation", "feature", &[], false, None),
            ]),
            policy::expected(
                "mixed",
                true,
                "optional",
                &["production_code"],
                &["documentation"],
                vec![
                    policy::obligation("code", "production_code", true, "optional", &[], BEHAVIOR),
                    policy::obligation(
                        "docs",
                        "documentation",
                        false,
                        "not_applicable",
                        &[],
                        PROPORTIONAL,
                    ),
                ],
            ),
        ),
        (
            policy::request(vec![
                policy::boundary("feature", "production_code", "feature", &[], false, None),
                policy::boundary(
                    "refactor",
                    "production_code",
                    "behavior_preserving_refactor",
                    &[],
                    false,
                    None,
                ),
            ]),
            policy::expected(
                "engineering",
                true,
                "optional",
                &["production_code", "production_code"],
                &[],
                vec![
                    policy::obligation(
                        "feature",
                        "production_code",
                        true,
                        "optional",
                        &[],
                        BEHAVIOR,
                    ),
                    policy::obligation(
                        "refactor",
                        "production_code",
                        true,
                        "optional",
                        &["green_or_characterization_baseline"],
                        BEHAVIOR,
                    ),
                ],
            ),
        ),
        case(
            policy::boundary("mandated", "production_code", "feature", &[], true, None),
            "engineering",
            true,
            "required",
            &["production_code"],
            &[],
            policy::obligation(
                "mandated",
                "production_code",
                true,
                "required",
                &["test_first_sequence"],
                BEHAVIOR,
            ),
        ),
    ];
    cases
        .into_iter()
        .try_for_each(|(request, expected)| policy::assert_v2(fixture.root(), request, expected))
}

const PROPORTIONAL: &[&str] = &["proportional_structural_or_behavioral_proof"];

fn case(
    boundary: serde_json::Value,
    classification: &str,
    tests_required: bool,
    mode: &str,
    tdd_boundaries: &[&str],
    proof_boundaries: &[&str],
    obligation: serde_json::Value,
) -> (serde_json::Value, serde_json::Value) {
    (
        policy::request(vec![boundary]),
        policy::expected(
            classification,
            tests_required,
            mode,
            tdd_boundaries,
            proof_boundaries,
            vec![obligation],
        ),
    )
}
