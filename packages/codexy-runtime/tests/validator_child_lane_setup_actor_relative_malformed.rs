use super::validator_child_lane_setup_actor_grammar::assert_result;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_fails_closed_for_malformed_direct_relative_markers() -> TestResult {
    for (label, setup, expected) in [
        (
            "malformed who chain cannot promote a parent report object",
            "The child who quickly parent told the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "malformed who chain cannot promote an orchestrator report object",
            "The child who quickly parent told the orchestrator did create branch codexy/463 after classification.",
            false,
        ),
        (
            "malformed which chain cannot promote a parent report object",
            "The child which quickly parent told the parent did create branch codexy/463 after classification.",
            false,
        ),
        (
            "malformed which chain cannot promote an orchestrator report object",
            "The child which quickly parent told the orchestrator did create branch codexy/463 after classification.",
            false,
        ),
        (
            "malformed chain with a child report object remains rejected",
            "The child who quickly parent told the child did create branch codexy/463 after classification.",
            false,
        ),
        (
            "valid who modifier chain retains relative ownership",
            "The parent who quickly told the child did create branch codexy/463 after classification.",
            true,
        ),
        (
            "valid which modifier chain retains relative ownership",
            "The parent which quickly told the child did create branch codexy/463 after classification.",
            true,
        ),
        (
            "absent relative syntax retains ordinary parent attribution",
            "The parent created branch codexy/463 after classification.",
            true,
        ),
    ] {
        assert_result(label, setup, expected)?;
    }
    Ok(())
}
