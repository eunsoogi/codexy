use serde_json::{Value, json};

#[path = "review_control_direct_state.rs"]
mod direct_state;
#[path = "post_cap_review_graph.rs"]
mod graph;
#[path = "post_cap_disposition_fixture.rs"]
mod disposition_fixture;

#[path = "final_disposition/fixture.rs"]
mod fixture;
#[path = "final_disposition/builder.rs"]
mod builder;
#[path = "final_disposition/handoff.rs"]
mod handoff;
#[path = "final_disposition/runner.rs"]
mod runner;
#[path = "final_disposition/runner_native_919.rs"]
mod runner_native_919;
#[path = "final_disposition/native_919.rs"]
mod native_919;

pub(crate) use handoff::{validate_handoff, validate_handoff_with_state};
pub(crate) use builder::build_pr_state;
pub(crate) use runner::{produce, produce_for, produce_with_fixture_mutation, produce_without_locator};
pub(crate) use builder::build_pr_state_with_states;
pub(crate) use handoff::validate_handoff_state;
pub(crate) use native_919::{
    final_control_919, recover_919, DELTA_EVENT as native_919_delta_event,
    FULL_EVENT as native_919_full_event, ISSUE as native_919_issue,
    PULL_REQUEST as native_919_pull_request,
    REMAINING_FINDING as native_919_remaining_finding,
    REQUIRED_EVENT as native_919_required_event,
};
pub(crate) use runner::canonical_third_predecessor;
pub(crate) use runner::produce_with_states;

const CASE_HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub(crate) fn repaired_head_control(issue: u64) -> Value {
    let mut control = direct_state::post_cap_control_with_evidence(
        issue,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
        "in_scope_contract_root_repair",
        direct_state::SYNTHETIC_REPAIR_EVIDENCE,
    );
    set_third_block(
        &mut control,
        direct_state::SYNTHETIC_REPAIR_EVIDENCE,
        "final-source-repair",
        "repair-current.txt",
    );
    control["reviewed_head"] = json!(direct_state::SYNTHETIC_REPAIR_EVIDENCE);
    control["unresolved_findings"] = control["terminal_review_history"][2]["unresolved_findings"].clone();
    control["post_cap_re_review"]["qualifying_change"]["to_head"] =
        json!(direct_state::SYNTHETIC_REPAIR_EVIDENCE);
    control["final_disposition"] = json!({
        "schema": "codexy.review-control-final-disposition.v1",
        "kind": "third_block_repair",
        "review_event_id": "strict-required-head-1",
        "source_head": direct_state::SYNTHETIC_REPAIR_EVIDENCE,
        "head_oid": direct_state::SYNTHETIC_CURRENT_HEAD,
        "base_oid": direct_state::SYNTHETIC_BASE,
        "addressed_finding_ids": ["final-source-repair"],
        "remaining_finding_ids": [],
        "source_repair": {
            "from_head": direct_state::SYNTHETIC_REPAIR_EVIDENCE,
            "evidence_commit": direct_state::SYNTHETIC_FINAL_EVIDENCE,
            "finding_ids": ["final-source-repair"]
        }
    });
    control
}

pub(crate) fn same_head_control(issue: u64) -> Value {
    let mut control = direct_state::post_cap_control_with_evidence(
        issue,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
        "in_scope_contract_root_repair",
        direct_state::SYNTHETIC_REPAIR_EVIDENCE,
    );
    set_third_block(
        &mut control,
        direct_state::SYNTHETIC_CURRENT_HEAD,
        "evidence-only-correction",
        "plugins/codexy/agents/codexy-sentinel.toml",
    );
    control["unresolved_findings"] = control["terminal_review_history"][2]["unresolved_findings"].clone();
    control["final_disposition"] = json!({
        "schema": "codexy.review-control-final-disposition.v1",
        "kind": "third_block_repair",
        "review_event_id": "strict-required-head-1",
        "source_head": direct_state::SYNTHETIC_CURRENT_HEAD,
        "head_oid": direct_state::SYNTHETIC_CURRENT_HEAD,
        "base_oid": direct_state::SYNTHETIC_BASE,
        "addressed_finding_ids": ["evidence-only-correction"],
        "remaining_finding_ids": [],
        "evidence_refresh": {
            "proof_head": direct_state::SYNTHETIC_CURRENT_HEAD,
            "finding_ids": ["evidence-only-correction"],
            "public_summaries": [{
                "finding_id": "evidence-only-correction",
                "status": "PASS",
                "case_hash": CASE_HASH
            }]
        }
    });
    control
}

pub(crate) fn third_block_predecessor(control: &Value) -> Value {
    let mut previous = control.clone();
    previous
        .as_object_mut()
        .expect("final disposition control object")
        .remove("final_disposition");
    previous["reviewed_head"] = previous["terminal_review_history"][2]["reviewed_head"].clone();
    previous["terminal_result"] = previous["terminal_review_history"][2]["terminal_result"].clone();
    previous["unresolved_findings"] =
        previous["terminal_review_history"][2]["unresolved_findings"].clone();
    previous
}

fn set_third_block(control: &mut Value, head: &str, id: &str, path: &str) {
    control["terminal_review_history"][2]["reviewed_head"] = json!(head);
    control["terminal_review_history"][2]["terminal_result"] = json!("BLOCK");
    control["terminal_review_history"][2]["unresolved_findings"] = json!([{
        "id": id,
        "path": path,
        "kind": "source_defect"
    }]);
    control["terminal_result"] = json!("BLOCK");
}
