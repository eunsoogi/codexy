use std::{path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

pub(crate) const CONTEXT_CONTRACT: &str = "skills/orchestration/references/context-tiers.json";
pub(crate) const CONTEXT_TASK_CLASSES: [&str; 10] = [
    "orchestration/lane setup",
    "implementation",
    "review response",
    "GitHub/merge",
    "validation/QA",
    "documentation/skill authoring",
    "plugin/release",
    "investigation/debugging",
    "issue/intake only",
    "other",
];

#[test]
fn context_contract_covers_tiers_profiles_and_safety_invariants() -> TestResult {
    const TIERS: [&str; 4] = ["always_on", "task_selected", "event_delta", "refresh_only"];
    const SAFETY_FIELDS: [&str; 10] = [
        "issue_pr_identity",
        "owner_worktree",
        "base_head_sha",
        "dirty_index_state",
        "checks",
        "unresolved_review_threads",
        "selected_reviewer_state",
        "verification",
        "external_gate",
        "next_action",
    ];
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let contract: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(CONTEXT_CONTRACT))?)?;
    assert_eq!(contract["schema"], "codexy.context-tiers.v1");
    assert_eq!(contract["tier_order"], json!(TIERS));
    assert_eq!(
        contract["routing"]["task_classes"],
        json!(CONTEXT_TASK_CLASSES)
    );
    for profile in ["light", "standard", "strict"] {
        let tiers = contract["profile_matrix"][profile]
            .as_object()
            .ok_or("profile tiers")?;
        assert_eq!(tiers.len(), TIERS.len(), "{profile} must decide every tier");
        assert!(TIERS.iter().all(|tier| tiers.contains_key(*tier)));
    }
    let fields = contract["retained_fields"]
        .as_array()
        .ok_or("retained fields")?;
    for required in SAFETY_FIELDS {
        let field = fields
            .iter()
            .find(|field| field["name"] == required)
            .ok_or_else(|| format!("missing safety field {required}"))?;
        assert_eq!(field["safety_invariant"], true);
        assert_eq!(field["budget_exempt"], true);
    }
    assert_eq!(
        contract["budget_semantics"]["cache_metadata"],
        "unavailable_not_zero"
    );
    Ok(())
}

pub(crate) fn assert_context_contract_rejected(
    root: &Path,
    path: &Path,
    baseline: &Value,
    mutate: impl FnOnce(&mut Value),
) -> TestResult {
    let mut invalid = baseline.clone();
    mutate(&mut invalid);
    std::fs::write(path, serde_json::to_vec(&invalid)?)?;
    assert!(
        !check_context_contract(root)?.status.success(),
        "invalid context contract passed"
    );
    std::fs::write(path, serde_json::to_vec(baseline)?)?;
    Ok(())
}

pub(crate) fn check_context_contract(root: &Path) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?)
}

#[path = "../validator_agent_model_assignments.rs"]
mod validator_agent_model_assignments;

#[path = "../validator_specialist_role_reduction.rs"]
mod validator_specialist_role_reduction;

#[path = "../validator_agent_registration.rs"]
mod validator_agent_registration;

#[path = "../validator_agent_registration_bootstrap.rs"]
mod validator_agent_registration_bootstrap;

#[path = "../validator_agent_registration_bootstrap_security.rs"]
mod validator_agent_registration_bootstrap_security;

#[path = "../validator_agent_registration_edges.rs"]
mod validator_agent_registration_edges;

#[path = "../validator_agent_registration_hardening.rs"]
mod validator_agent_registration_hardening;

#[path = "../validator_agent_registration_lifecycle.rs"]
mod validator_agent_registration_lifecycle;

#[path = "../validator_agent_registration_security.rs"]
mod validator_agent_registration_security;

#[path = "../validator_agent_registration_transactions.rs"]
mod validator_agent_registration_transactions;

#[path = "../validator_github_agent_activation.rs"]
mod validator_github_agent_activation;

#[path = "../validator_prompt_metadata.rs"]
mod validator_prompt_metadata;

#[path = "../validator_devtools_metadata.rs"]
mod validator_devtools_metadata;

#[path = "../validator_child_routing_policy.rs"]
mod validator_child_routing_policy;

#[path = "../validator_child_routing_contract.rs"]
mod validator_child_routing_contract;

#[path = "../validator_tdd_classification_policy.rs"]
mod validator_tdd_classification_policy;

#[path = "../validator_context_tiers.rs"]
mod validator_context_tiers;

#[path = "../validator_task_surface_routing.rs"]
mod validator_task_surface_routing;

#[path = "../validator_project_neutral_core.rs"]
mod validator_project_neutral_core;

#[path = "../validator_portable_governed_code.rs"]
mod validator_portable_governed_code;

#[path = "../validator_review_control.rs"]
mod validator_review_control;

#[path = "../validator_routing_measurement.rs"]
mod validator_routing_measurement;

#[path = "../validator_routing_measurement_promotions.rs"]
mod validator_routing_measurement_promotions;

#[path = "../validator_routing_measurement_schema.rs"]
mod validator_routing_measurement_schema;

#[path = "../validator_prompt_host_limits.rs"]
mod validator_prompt_host_limits;

#[path = "../skill_reference_links.rs"]
mod skill_reference_links;

#[path = "../workflow_profile_contract.rs"]
mod workflow_profile_contract;

#[path = "../workflow_profile_active_markdown.rs"]
mod workflow_profile_active_markdown;

#[path = "../workflow_profile_signals.rs"]
mod workflow_profile_signals;

#[path = "../workflow_profile_grammar.rs"]
mod workflow_profile_grammar;

#[path = "../validator_roles.rs"]
mod validator_roles;

#[path = "../validator_review_handoff.rs"]
mod validator_review_handoff;

#[path = "../validator_pr_state_capture.rs"]
mod validator_pr_state_capture;
