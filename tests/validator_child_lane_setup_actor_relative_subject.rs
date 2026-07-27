use super::validator_child_lane_setup_actor_grammar::assert_result;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_keeps_whose_relative_report_roles_out_of_the_main_subject() -> TestResult {
    for (label, setup, expected) in [
        (
            "whose relative report child object does not replace the parent subject",
            "The parent whose orchestrator told the child did create branch codexy/463 after classification.",
            true,
        ),
        (
            "whose relative report parent object does not replace the child subject",
            "The child whose orchestrator told the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "whose present relative report child object does not replace the parent subject",
            "The parent whose orchestrator tells the child does create branch codexy/463 after classification.",
            true,
        ),
        (
            "whose present relative report parent object does not replace the child subject",
            "The child whose orchestrator tells the parent does create branch codexy/463 after classification.",
            false,
        ),
        (
            "a completed whose relative clause permits a new child main subject",
            "The parent whose orchestrator reviewed it says the child did create branch codexy/463 after classification.",
            false,
        ),
        (
            "a completed whose relative clause permits a new parent main subject",
            "The child whose orchestrator reviewed it says the parent did create branch codexy/463 after classification.",
            true,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}
