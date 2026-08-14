use serde_json::{Value, json};

const CONTRACT_SHA256: &str = "30e2a0c55aa2db0a84e6924f5a4731f335ea652f79123af992903d8ec1c617e2";

pub(super) fn bind_ledger(state: &mut Value) {
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
        event["issue_contract_sha256"] = json!(CONTRACT_SHA256);
    }
}
