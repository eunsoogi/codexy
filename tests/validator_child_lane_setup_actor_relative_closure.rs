use super::validator_child_lane_setup_actor_grammar::assert_result;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_closes_valid_direct_relatives_before_later_main_reports() -> TestResult {
    for (label, setup, expected) in [
        (
            "who relative closes before a later child main report",
            "The parent who quickly told the orchestrator says the child did create branch codexy/463 after classification.",
            false,
        ),
        (
            "who relative closes before a later parent main report",
            "The child who quickly told the orchestrator says the parent did create branch codexy/463 after classification.",
            true,
        ),
        (
            "which relative closes before a later child main report",
            "The parent which quickly told the orchestrator says the child did create branch codexy/463 after classification.",
            false,
        ),
        (
            "which relative closes before a later parent main report",
            "The child which quickly told the orchestrator says the parent did create branch codexy/463 after classification.",
            true,
        ),
        (
            "perhaps modifier keeps a who relative closed",
            "The parent who perhaps told the orchestrator says the child did create branch codexy/463 after classification.",
            false,
        ),
        (
            "direct main setup follows a closed who relative",
            "The child who quickly told the orchestrator did create branch codexy/463 after classification.",
            false,
        ),
        (
            "malformed direct marker remains fail closed",
            "The child who quickly parent told the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "no new subject inherits the child main subject",
            "The child who quickly told the orchestrator said and then did create branch codexy/463 after classification.",
            false,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}
