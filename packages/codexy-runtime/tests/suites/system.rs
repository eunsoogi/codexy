#[path = "../architecture_docs_inventory.rs"]
mod architecture_docs_inventory;
#[path = "../plugin_product_boundary_contract.rs"]
mod plugin_product_boundary_contract;

#[path = "../codegraph_language_regressions.rs"]
mod codegraph_language_regressions;

#[path = "../codegraph_web_language_regressions.rs"]
mod codegraph_web_language_regressions;

#[path = "../validator_getcodexy_component_contract.rs"]
mod validator_getcodexy_component_contract;

#[path = "../integration_target_budget.rs"]
mod integration_target_budget;

#[path = "../lsp_readiness.rs"]
mod lsp_readiness;

#[path = "../mcp_response_checker.rs"]
mod mcp_response_checker;

#[path = "../mcp_server_names.rs"]
mod mcp_server_names;

#[path = "../mcp_stdio.rs"]
mod mcp_stdio;

#[path = "../mcp_wrapper_bootstrap.rs"]
mod mcp_wrapper_bootstrap;

#[path = "../mcp_runtime_deduplication.rs"]
mod mcp_runtime_deduplication;

#[path = "../module_owned_package_roots.rs"]
mod module_owned_package_roots;

#[path = "../repository_eol_contract.rs"]
mod repository_eol_contract;

#[path = "../rust_workflow_budget.rs"]
mod rust_workflow_budget;

#[path = "../version_bump_pr_readiness.rs"]
mod version_bump_pr_readiness;

#[path = "../version_bump_pr_snapshot_refresh.rs"]
mod version_bump_pr_snapshot_refresh;

#[path = "../version_bump_pr_metadata_failures.rs"]
mod version_bump_pr_metadata_failures;

#[path = "../version_bump_reconciliation_state_machine.rs"]
mod version_bump_reconciliation_state_machine;

#[path = "../version_bump_governing_identity.rs"]
mod version_bump_governing_identity;

#[path = "../version_bump_pr_state.rs"]
mod version_bump_pr_state;

#[path = "../version_bump_pr_test_support.rs"]
mod version_bump_pr_test_support;

#[path = "../version_bump_workflow_structure.rs"]
mod version_bump_workflow_structure;

#[path = "../version_bump_workflow_contract.rs"]
mod version_bump_workflow_contract;

#[path = "../version_bump_workflow_model.rs"]
mod version_bump_workflow_model;

#[path = "../version_bump_workflow_topology_matrix.rs"]
mod version_bump_workflow_topology_matrix;

#[path = "../support/version_pr_workflow_fixture.rs"]
mod version_pr_workflow_fixture;

#[path = "../version_bump_workflow_adapter.rs"]
mod version_bump_workflow_adapter;

#[path = "../session_audit_custom_tools.rs"]
mod session_audit_custom_tools;

#[path = "../session_audit_event_runtime.rs"]
mod session_audit_event_runtime;

#[path = "../skill_boundary_taxonomy.rs"]
mod skill_boundary_taxonomy;

#[path = "../session_audit_parent_bounds.rs"]
mod session_audit_parent_bounds;

pub(crate) mod stage_budget_test_support {
    use serde_json::{Value, json};

    pub(crate) fn set(value: &mut Value, path: &str, replacement: Value) {
        let path = format!("/{}", path.replace('.', "/"));
        *value.pointer_mut(&path).unwrap() = replacement;
    }

    pub(crate) fn fixture() -> Value {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../../plugins/codexy/skills/orchestration/templates/stage-budget-receipt.json"
        ))
        .unwrap();
        value["limits"] = json!({"contextBytes":1000,"toolOutputBytes":1000,"replayEvents":10,"turns":10,"toolCalls":10});
        value["usage"] = json!({"contextBytes":100,"toolOutputBytes":100,"turns":1,"toolCalls":1});
        value["measures"]["toolOutputBytes"]["value"] = json!(100);
        value["identity"]["stable"] = json!("stage-601");
        value["identity"]["volatile"] = json!("event-1");
        value["events"]["identities"] = json!(["event-1"]);
        value
    }

    pub(crate) fn oversized(value: &mut Value, kind: &str, identity: &str, bytes: u64, state: &str) {
        let result = json!({"kind":kind,"identity":identity,"bytes":bytes,"state":state,"bodyReplayed":false});
        set(value, "oversizedResult", result);
        if kind == "tool-output" {
            set(value, "usage.toolOutputBytes", json!(bytes));
            set(value, "measures.toolOutputBytes.value", json!(bytes));
        } else {
            set(value, "usage.contextBytes", json!(bytes));
        }
    }

    pub(crate) fn previous_anchor(value: &Value) -> Value {
        json!({"stage":value["stage"],"stageSequence":value["stageSequence"],"previousReceiptIdentity":value["previousReceiptIdentity"],"receiptIdentity":value["receiptIdentity"],"owner":value["owner"],"identity":value["identity"],"safety":value["safety"],"proof":value["proof"],"limits":value["limits"],"usage":value["usage"],"events":value["events"],"oversizedResult":value["oversizedResult"],"cumulativeReplayEvents":value["continuity"]["cumulativeReplayEvents"]})
    }

    pub(crate) fn declare(value: &mut Value, decision: &str) {
        value["decision"] = json!(decision);
        let action = match (decision, value["stage"].as_str()) {
            ("continue", Some("wait" | "selected-review")) => "wait-for-event",
            ("continue", _) => "continue-stage",
            ("compact", _) => "compact-context",
            _ => "handoff-parent",
        };
        value["nextAction"] = json!(action);
    }
}

#[path = "../session_audit_receipt.rs"]
mod session_audit_receipt;

#[path = "../session_audit_receipt_contract.rs"]
mod session_audit_receipt_contract;

#[path = "../session_audit_receipt_review_feedback.rs"]
mod session_audit_receipt_review_feedback;

#[path = "../session_audit_stage_budget.rs"]
mod session_audit_stage_budget;

#[path = "../session_audit_stage_budget_continuity.rs"]
mod session_audit_stage_budget_continuity;

#[path = "../validator_lsp_readiness.rs"]
mod validator_lsp_readiness;

#[path = "../validator_marketplace_publish_contract.rs"]
mod validator_marketplace_publish_contract;

#[path = "../validator_marketplace_publish_permissions.rs"]
mod validator_marketplace_publish_permissions;

#[path = "../validator_mcp.rs"]
mod validator_mcp;

#[path = "../validator_removed_mcp.rs"]
mod validator_removed_mcp;

#[path = "../validator_runtime_contract.rs"]
mod validator_runtime_contract;

#[path = "../worktree_reservation_harness.rs"]
mod worktree_reservation_harness;
