use std::fs;

use serde_json::json;

use crate::support::TestResult;

#[test]
fn pr_ready_handoff_requires_an_explicit_review_decision() -> TestResult {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state = temp.path().join("state.json");
    fs::write(&handoff, "PR-ready: handoff artifact attached for parent.\n")?;
    let state_json = json!({
        "number":128, "state":"OPEN", "isDraft":true, "mergeStateStatus":"CLEAN", "headRefOid":"head",
        "reviewThreads":{"nodes":[]},
        "reviewProfile":"standard",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"head","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed","event_id":"e-passed","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-full","predecessor_event_id":null,"profile":"standard","base_oid":"base","head_oid":"head","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-passed","predecessor_event_id":"e-full","profile":"standard","base_oid":"base","head_oid":"head","state":"passed","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}
        ]}
    });
    for decision in [None, Some("CHANGES_REQUESTED")] {
        let mut state_json = state_json.clone();
        if let Some(decision) = decision {
            state_json["reviewDecision"] = json!(decision);
        }
        fs::write(&state, serde_json::to_vec(&state_json)?)?;
        let output = crate::support::validator_completion_handoff_files(&handoff, &state)?;
        assert!(
            !output.status.success(),
            "a PR-ready handoff without a matching reviewDecision must fail closed"
        );
    }
    Ok(())
}
