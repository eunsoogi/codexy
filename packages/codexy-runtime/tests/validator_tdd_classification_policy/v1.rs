use serde_json::json;

use super::policy_support as policy;
use crate::support::TestResult;

#[test]
fn resolver_classifies_engineering_non_engineering_and_mixed_boundaries() -> TestResult {
    let fixture = policy::fixture()?;
    for (boundaries, expected) in [
        (
            json!(["production_code"]),
            json!({"classification":"engineering","engineering_tdd_required":true,"tdd_boundaries":["production_code"],"proportional_proof_boundaries":[]}),
        ),
        (
            json!(["runtime_behavior", "validator", "hook", "cli", "workflow"]),
            json!({"classification":"engineering","engineering_tdd_required":true,"tdd_boundaries":["runtime_behavior","validator","hook","cli","workflow"],"proportional_proof_boundaries":[]}),
        ),
        (
            json!(["markdown_backed_parser"]),
            json!({"classification":"engineering","engineering_tdd_required":true,"tdd_boundaries":["markdown_backed_parser"],"proportional_proof_boundaries":[]}),
        ),
        (
            json!(["readme", "documentation", "instruction_only_skill"]),
            json!({"classification":"non_engineering","engineering_tdd_required":false,"tdd_boundaries":[],"proportional_proof_boundaries":["readme","documentation","instruction_only_skill"]}),
        ),
        (
            json!([
                "declarative_metadata",
                "diagram",
                "roadmap_or_release_prose"
            ]),
            json!({"classification":"non_engineering","engineering_tdd_required":false,"tdd_boundaries":[],"proportional_proof_boundaries":["declarative_metadata","diagram","roadmap_or_release_prose"]}),
        ),
        (
            json!(["validator", "documentation"]),
            json!({"classification":"mixed","engineering_tdd_required":true,"tdd_boundaries":["validator"],"proportional_proof_boundaries":["documentation"]}),
        ),
    ] {
        policy::assert_v1(fixture.root(), boundaries, expected)?;
    }
    Ok(())
}

#[test]
fn resolver_rejects_unknown_or_incomplete_machine_owned_requests() -> TestResult {
    let fixture = policy::fixture()?;
    for request in [
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":[]}),
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":["markdown"]}),
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":["documentation","markdown"]}),
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":["documentation","documentation"]}),
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":["documentation"],"unexpected":true}),
        json!({"schema":"other","boundaries":["documentation"]}),
    ] {
        policy::assert_rejected(
            fixture.root(),
            request,
            "invalid v1 classification request passed",
        )?;
    }
    Ok(())
}
