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

#[path = "../validator_tdd_classification_policy.rs"]
mod validator_tdd_classification_policy;

#[path = "../validator_project_neutral_core.rs"]
mod validator_project_neutral_core;

#[path = "../validator_portable_governed_code.rs"]
mod validator_portable_governed_code;

#[path = "../validator_review_control.rs"]
mod validator_review_control;

#[path = "../validator_review_control_disposition.rs"]
mod validator_review_control_disposition;

#[path = "../validator_review_control_lifecycle.rs"]
mod validator_review_control_lifecycle;

#[path = "../validator_review_control_import.rs"]
mod validator_review_control_import;

#[path = "../validator_review_control_import_contract.rs"]
mod validator_review_control_import_contract;

#[path = "../validator_review_control_identity.rs"]
mod validator_review_control_identity;

#[path = "../validator_completion_handoff_review_identity.rs"]
mod validator_completion_handoff_review_identity;

#[path = "../validator_review_control_migration.rs"]
mod validator_review_control_migration;

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

#[path = "../validator_post_cap_review.rs"]
mod validator_post_cap_review;

#[path = "../validator_post_cap_review_negatives.rs"]
mod validator_post_cap_review_negatives;

#[path = "../validator_post_cap_external_finding.rs"]
#[cfg(unix)]
mod validator_post_cap_external_finding;

#[path = "../validator_post_cap_external_finding_negatives.rs"]
#[cfg(unix)]
mod validator_post_cap_external_finding_negatives;

#[path = "../validator_post_cap_finding_disposition.rs"]
#[cfg(unix)]
mod validator_post_cap_finding_disposition;

#[path = "../validator_post_cap_finding_disposition_handoff.rs"]
#[cfg(unix)]
mod validator_post_cap_finding_disposition_handoff;
