use std::path::Path;

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

const HEAD: &str = "32b03a210b3defb2d29dd352283ea2488e60d893";

#[test]
fn validator_rejects_open_pr_completion_claims_even_with_sentinel_evidence() -> TestResult {
    for handoff in [
        format!("Completed. Sentinel: PASS on current head {HEAD}.\n"),
        format!("Finished. Sentinel: BLOCK on current head {HEAD}.\n"),
        format!("Finalized. Sentinel: UNOBSERVABLE after bounded wait on current head {HEAD}.\n"),
    ] {
        reject_open_pr_completion_handoff(&handoff)?;
    }
    Ok(())
}

#[test]
fn validator_keeps_deferrals_and_readiness_distinct_from_completion() -> TestResult {
    for handoff in [
        format!(
            "Maintainer requested no-merge; Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. Work is complete after PR #128.\n"
        ),
        format!(
            "PR ready for parent handoff. Packaged Codexy Sentinel Turing: PASS on current head {HEAD}.\n"
        ),
    ] {
        accept_open_pr_handoff(&handoff)?;
    }
    Ok(())
}

#[test]
fn validator_requires_typed_review_for_pr_and_parent_ready_handoffs() -> TestResult {
    for handoff in ["PR ready for parent handoff.\n", "Parent handoff ready.\n"] {
        assert!(!validate_readiness_handoff(handoff, json!({}))?.status.success());
        let strict = validate_readiness_handoff(handoff, strict_review())?;
        assert!(strict.status.success(), "{}", String::from_utf8_lossy(&strict.stderr));
        assert!(validate_readiness_handoff(
            handoff,
            json!({"reviewProfile":"light","reviewDecision":"NOT_REQUIRED"})
        )?
        .status
        .success());
    }
    Ok(())
}

fn reject_open_pr_completion_handoff(handoff: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        !output.status.success(),
        "validator should reject open-PR completion handoff\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("opening a PR is not completion"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn accept_open_pr_handoff(handoff: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        output.status.success(),
        "validator should accept non-completion or explicitly deferred handoff\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_completion_handoff(handoff_path: &Path, pr_state_path: &Path) -> OutputResult {
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}

fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    let mut state: serde_json::Value = serde_json::from_str(&format!(
            r###"{{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/221-sentinel-bounded-wait-status","headRefOid":"{HEAD}","localHeadOid":"{HEAD}","remoteHeadOid":"{HEAD}","reviewProfile":"strict","reviewEvidence":{{"schema":"codexy.review-readiness.v1","head_oid":"{HEAD}","profile":"strict","reviewer":{{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"}},"state":"passed","event_id":"e-passed","blockers":[]}},"reviewLedger":{{"schema":"codexy.review-ledger.v1","events":[{{"id":"e-full","predecessor_event_id":null,"profile":"strict","base_oid":"base","head_oid":"{HEAD}","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}},{{"id":"e-passed","predecessor_event_id":"e-full","profile":"strict","base_oid":"base","head_oid":"{HEAD}","state":"passed","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}}]}},"worktreeStatus":"## codexy/221-sentinel-bounded-wait-status...origin/codexy/221-sentinel-bounded-wait-status","latestReviews":[{{"body":"Didn't find any major issues.\n\nReviewed commit: `{HEAD}`","author":{{"login":"automated-review"}},"submittedAt":"2026-07-03T00:00:00Z","commit":{{"oid":"{HEAD}"}}}}],"reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}}}"###
    ))?;
    bind_ledger(&mut state);
    crate::support::review_control_state::namespace_review_control(&mut state);
    std::fs::write(&pr_state_path, serde_json::to_vec(&state)?)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}

fn validate_readiness_handoff(handoff: &str, review: serde_json::Value) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    let mut state = json!({
        "number":128, "state":"OPEN", "isDraft":false, "mergeStateStatus":"CLEAN", "reviewDecision":"APPROVED",
        "headRefName":"codexy/221-sentinel-bounded-wait-status",
        "headRefOid":HEAD, "localHeadOid":HEAD, "remoteHeadOid":HEAD,
        "worktreeStatus":"## codexy/221-sentinel-bounded-wait-status...origin/codexy/221-sentinel-bounded-wait-status",
        "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
    });
    state.as_object_mut().expect("state").extend(
        review
            .as_object()
            .expect("review")
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    crate::support::review_control_state::namespace_review_control(&mut state);
    std::fs::write(&pr_state_path, serde_json::to_vec(&state)?)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}

fn strict_review() -> serde_json::Value {
    let mut review = json!({
        "reviewProfile":"strict",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":HEAD,"profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"passed","event_id":"e-passed","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-full","predecessor_event_id":null,"profile":"strict","base_oid":"base","head_oid":HEAD,"state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-passed","predecessor_event_id":"e-full","profile":"strict","base_oid":"base","head_oid":HEAD,"state":"passed","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}
        ]}
    });
    bind_ledger(&mut review);
    review
}

fn bind_ledger(state: &mut Value) {
    for event in state["reviewLedger"]["events"]
        .as_array_mut()
        .expect("review ledger events")
    {
        event["issue_contract"] = json!({
            "problem":"owned problem",
            "scope":"owned scope",
            "acceptance_criteria":[{"id":"ac-1"}],
            "owned_invariant_ids":[],
            "exclusions":[],
            "adjacent_dependencies":[]
        });
        event["issue_contract_sha256"] = json!("30e2a0c55aa2db0a84e6924f5a4731f335ea652f79123af992903d8ec1c617e2");
    }
}
