use std::fs;

use serde_json::json;

use crate::support::TestResult;

const HEAD: &str = "32b03a210b3defb2d29dd352283ea2488e60d893";
const BASE: &str = "0000000000000000000000000000000000000000";

#[test]
fn pr_ready_handoff_requires_an_explicit_review_decision() -> TestResult {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state = temp.path().join("state.json");
    for handoff_text in ["PR-ready: yes\n", "Merge-ready: yes\n"] {
        fs::write(&handoff, handoff_text)?;
        for decision in [None, Some("CHANGES_REQUESTED")] {
            let mut state_json = valid_standard_state();
            match decision {
                Some(decision) => state_json["reviewControl"]["decision"] = json!(decision),
                None => { state_json["reviewControl"].as_object_mut().expect("control").remove("decision"); }
            }
            fs::write(&state, serde_json::to_vec(&state_json)?)?;
            let output = crate::support::validator_completion_handoff_files(&handoff, &state)?;
            assert!(
                !output.status.success(),
                "an affirmative current readiness label without a matching reviewDecision must fail closed: {handoff_text:?}"
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("review decision"),
                "the typed decision boundary was not reached: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

#[test]
fn negated_readiness_labels_do_not_require_review_decision() -> TestResult {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state = temp.path().join("state.json");
    fs::write(&state, serde_json::to_vec(&valid_pr_state())?)?;
    for handoff_text in ["PR-ready: no\n", "Merge-ready: no\n"] {
        fs::write(&handoff, handoff_text)?;
        let output = crate::support::validator_completion_handoff_files(&handoff, &state)?;
        assert!(output.status.success(), "negative label became a claim: {handoff_text:?}");
    }
    Ok(())
}

fn valid_standard_state() -> serde_json::Value {
    let mut state = valid_pr_state();
    state["reviewControl"] = json!({"schema":"codexy.review-control-state.v1","profile":"standard","decision":"APPROVED","evidence":{"schema":"codexy.review-readiness.v1","head_oid":HEAD,"profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed","event_id":"e-passed","blockers":[]},"ledger":{"schema":"codexy.review-ledger.v1","events":[
        {"id":"e-full","predecessor_event_id":null,"profile":"standard","base_oid":BASE,"head_oid":HEAD,"state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"issue_contract":{"problem":"synthetic problem","scope":"synthetic scope","acceptance_criteria":[{"id":"synthetic-ac-1"}],"owned_invariant_ids":[],"exclusions":[],"adjacent_dependencies":[]},"issue_contract_sha256":"9ed099f9e4430ae71459275cb6c48e48fb9bce80b802c0557b438cb50d95cbca","escalation":null},
        {"id":"e-passed","predecessor_event_id":"e-full","profile":"standard","base_oid":BASE,"head_oid":HEAD,"state":"passed","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"issue_contract":{"problem":"synthetic problem","scope":"synthetic scope","acceptance_criteria":[{"id":"synthetic-ac-1"}],"owned_invariant_ids":[],"exclusions":[],"adjacent_dependencies":[]},"issue_contract_sha256":"9ed099f9e4430ae71459275cb6c48e48fb9bce80b802c0557b438cb50d95cbca","escalation":null}
    ]}});
    state
}

fn valid_pr_state() -> serde_json::Value {
    json!({
        "number":128, "state":"OPEN", "isDraft":false, "mergeStateStatus":"CLEAN", "headRefName":"codexy/test",
        "headRefOid":HEAD, "localHeadOid":HEAD, "remoteHeadOid":HEAD,
        "worktreeStatus":"## codexy/test...origin/codexy/test", "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
    })
}
