use crate::support::TestResult;
use serde_json::{Value, json};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/final_disposition.rs"]
mod final_support;

const WITNESS_ISSUE: u64 = 919;
const WITNESS_PR: u64 = 989;
const WITNESS_BASE: &str = "fa12bcacf84e00ec4a66f4d8cd9d9f1693f83d9c";
const WITNESS_FULL: &str = "e8df6c7a1e2a43c544f106c08e9ce140f0e9f6ca";
const WITNESS_DELTA: &str = "6419fdeba270c2b889ee1986bdcda8642d0844586";
const WITNESS_HEAD: &str = "9e431bcfcae06227ab0c9ce0986f7edea709dcc6";

#[test]
fn current_919_witness_consumes_sanitized_same_head_disposition() -> TestResult {
    let control = witness_control();
    let previous = final_support::third_block_predecessor(&control);
    let produced = final_support::produce_for(&control, &previous, WITNESS_PR)?;

    assert_eq!(produced["issue_number"], WITNESS_ISSUE);
    assert_eq!(produced["terminal_review_count"], 3);
    assert_eq!(produced["terminal_review_history"].as_array().map(Vec::len), Some(3));
    for (event, head) in [
        (&produced["terminal_review_history"][0], WITNESS_FULL),
        (&produced["terminal_review_history"][1], WITNESS_DELTA),
        (&produced["terminal_review_history"][2], WITNESS_HEAD),
    ] {
        assert_eq!(event["terminal_result"], "BLOCK");
        assert_eq!(event["reviewed_head"], head);
        assert!(event.get("unresolved_findings").is_some());
    }
    assert_eq!(produced["final_disposition"]["head_oid"], WITNESS_HEAD);
    assert!(produced["final_disposition"].get("raw_cases").is_none());
    assert!(produced["final_disposition"].get("private_cases").is_none());

    let handoff = final_support::validate_handoff(&produced, WITNESS_PR, WITNESS_HEAD)?;
    assert!(
        handoff.status.success(),
        "#919 same-head disposition must pass direct handoff: {}",
        String::from_utf8_lossy(&handoff.stderr)
    );
    Ok(())
}

fn witness_control() -> Value {
    let mut control = final_support::same_head_control(WITNESS_ISSUE);
    control["terminal_review_history"][0]["reviewed_head"] = json!(WITNESS_FULL);
    control["terminal_review_history"][0]["terminal_result"] = json!("BLOCK");
    control["terminal_review_history"][1]["reviewed_head"] = json!(WITNESS_DELTA);
    control["terminal_review_history"][1]["unresolved_findings"] = json!([{
        "id": "919-canonical-private-replay",
        "path": "plugins/codexy/skills/project-brief/SKILL.md",
        "kind": "evidence_refresh"
    }]);
    control["terminal_review_history"][2]["reviewed_head"] = json!(WITNESS_HEAD);
    control["terminal_review_history"][2]["unresolved_findings"] = json!([{
        "id": "919-canonical-private-replay",
        "path": "plugins/codexy/skills/project-brief/SKILL.md",
        "kind": "evidence_refresh"
    }]);
    control["reviewed_head"] = json!(WITNESS_HEAD);
    control["unresolved_findings"] = control["terminal_review_history"][2]["unresolved_findings"].clone();
    control["post_cap_re_review"]["prior_reviewed_head"] = json!(WITNESS_DELTA);
    control["post_cap_re_review"]["qualifying_change"]["from_head"] = json!(WITNESS_DELTA);
    control["post_cap_re_review"]["qualifying_change"]["to_head"] = json!(WITNESS_HEAD);
    control["post_cap_re_review"]["qualifying_change"]["evidence_commit"] = json!(WITNESS_HEAD);
    control["post_cap_re_review"]["qualifying_change"]["finding_ids"] =
        json!(["919-canonical-private-replay"]);
    control["final_disposition"]["source_head"] = json!(WITNESS_HEAD);
    control["final_disposition"]["head_oid"] = json!(WITNESS_HEAD);
    control["final_disposition"]["base_oid"] = json!(WITNESS_BASE);
    control["final_disposition"]["evidence_refresh"]["proof_head"] = json!(WITNESS_HEAD);
    control["final_disposition"]["evidence_refresh"]["finding_ids"] =
        json!(["919-canonical-private-replay"]);
    control["final_disposition"]["addressed_finding_ids"] =
        json!(["919-canonical-private-replay"]);
    control["final_disposition"]["evidence_refresh"]["public_summaries"][0]["finding_id"] =
        json!("919-canonical-private-replay");
    control
}
