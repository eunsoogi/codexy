use serde_json::{Value, json};

const SYNTHETIC_CONTRACT_SHA256: &str = "9ed099f9e4430ae71459275cb6c48e48fb9bce80b802c0557b438cb50d95cbca";

pub(super) fn bind_ledger(state: &mut Value) {
    for event in state["reviewLedger"]["events"]
        .as_array_mut()
        .expect("review ledger events")
    {
        event["issue_contract"] = json!({
            "problem":"synthetic problem",
            "scope":"synthetic scope",
            "acceptance_criteria":[{"id":"synthetic-ac-1"}],
            "owned_invariant_ids":[],
            "exclusions":[],
            "adjacent_dependencies":[]
        });
        event["issue_contract_sha256"] = json!(SYNTHETIC_CONTRACT_SHA256);
    }
}
