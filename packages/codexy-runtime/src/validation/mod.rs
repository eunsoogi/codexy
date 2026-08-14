mod agent_model_contract;
mod agent_registration;
mod agent_registration_catalog;
mod child_goal_blocked_audit;
mod child_goal_reporting;
mod child_handoff_readiness;
mod child_handoff_readiness_claims;
mod child_handoff_readiness_heads;
mod child_handoff_readiness_status;
mod child_handoff_readiness_text;
#[path = "child_lane_active_threads_module.rs"]
mod child_lane_active_threads;
mod child_lane_classification_authority;
mod child_lane_classification_boundaries;
mod child_lane_classification_control;
mod child_lane_classification_fields;
mod child_lane_classification_schema;
mod child_lane_classification_setup;
mod child_lane_classification_setup_actions;
mod child_lane_classification_setup_actor;
mod child_lane_classification_setup_attribution;
mod child_lane_classification_setup_clause;
mod child_lane_classification_setup_condition;
mod child_lane_classification_setup_context;
mod child_lane_classification_setup_phrase;
mod child_lane_classification_setup_relations;
mod child_lane_classification_setup_relative;
mod child_lane_colon_classification_block;
mod child_lane_gfm_classification_table;
mod child_lane_owner_decision;
mod child_lane_ownership;
mod child_lane_ownership_fixes;
mod child_lane_ownership_phrases;
mod child_lane_ownership_recovery;
mod child_lane_ownership_setup;
mod child_lane_ownership_setup_markers;
mod child_lane_ownership_subagent_format;
mod child_lane_ownership_subagents;
mod child_lane_thread_tool_handler_capture;
mod child_lane_thread_tool_handler_defect_capture;
mod child_lane_thread_tool_handler_exact_error;
mod child_lane_thread_tool_handler_issue_reference;
mod child_lane_thread_tool_handler_issue_tracking;
mod child_lane_thread_tool_handler_issue_value;
mod child_lane_thread_tool_handler_lane_header;
mod child_lane_thread_tool_handler_lane_mentions;
mod child_lane_thread_tool_handler_no_route;
mod child_lane_thread_tool_handler_raw_lane;
mod child_lane_thread_tool_handler_route_owner_absence;
mod child_lane_thread_tool_handler_route_value;
mod child_lane_thread_tool_handler_scope;
mod child_lane_thread_tool_handler_scope_labels;
mod child_lane_thread_tool_handlers;
mod child_lane_thread_tools;
mod child_lifecycle_events;
mod child_terminal_handoff;
mod completion_handoff;
mod completion_handoff_compaction;
mod completion_handoff_loc_polarity;
mod completion_handoff_loc_remediation;
mod completion_handoff_pending_worktree;
mod completion_handoff_pending_worktree_labels;
mod completion_handoff_pending_worktree_search;
mod completion_handoff_pending_worktree_segments;
mod completion_handoff_pending_worktree_text;
mod completion_handoff_waiting;
mod conventional_commit;
mod custom_agent_mcp;
mod custom_agent_mcp_tools;
mod custom_agent_schema;
mod getcodexy_component_contract;
mod github_labels;
mod handoff_claims;
mod hooks;
mod issue_intake;
mod issue_intake_receipt;
mod lsp;
mod manifest;
mod markdown;
mod mcp;
mod mcp_required;
mod mcp_runtime;
mod merge_authorization;
mod merge_authorization_contract;
mod merge_authorization_json;
mod merge_message;
mod mode;
mod mode_dispatch;
mod prompt_yaml;
mod readiness_context;
mod readiness_state;
mod release_publish_contract;
mod removed_mcp;
mod repository_skill_root;
mod review;
mod review_control;
mod review_thread_evidence;
mod review_thread_readiness;
mod review_thread_resolution;
mod review_thread_waiting;
mod review_thread_waiting_phrases;
mod review_thread_waiting_refs;
mod roles;
mod roles_yaml;
mod routing_json;
mod routing_measurement;
mod routing_measurement_schema;
mod routing_policy;
mod runtime;
mod runtime_candidate_manifest;
mod runtime_release_contract;
mod runtime_release_schema;
mod tdd_classification;
mod touched_loc;
mod touched_loc_remediation;
mod value_arrays;
mod workflow_profile_evidence;
mod workflow_profile_grammar;
mod workflow_profiles;

use std::path::Path;

use anyhow::Result;

pub use mode::Mode;
pub use mode_dispatch::{errors, run};
pub(super) use value_arrays::{json_array_strings, toml_array_strings};

pub(crate) fn validate_getcodexy_component_contract(plugin_root: &Path) -> Result<()> {
    getcodexy_component_contract::validate(plugin_root).map_err(anyhow::Error::msg)
}

/// Returns the LSP file extensions covered by Codexy validation metadata.
pub fn covered_extensions(plugin_root: &Path) -> Result<Vec<String>> {
    lsp::covered_extensions(&devtools_root(plugin_root))
}

fn devtools_root(plugin_root: &Path) -> std::path::PathBuf {
    if plugin_root.file_name().is_some_and(|name| name == "codexy") {
        return plugin_root.parent().map_or_else(
            || std::path::PathBuf::from("plugins/codexy-devtools"),
            |parent| parent.join("codexy-devtools"),
        );
    }
    plugin_root.to_path_buf()
}

/// Returns touched-LOC diagnostics for an explicit repository root.
pub fn touched_loc_diagnostics(root: &Path, base_ref: &str) -> Result<Vec<String>> {
    touched_loc::diagnostics_at(root, base_ref)
}

/// Lists reproducible readability-audit classifications for governed sources.
pub fn density_inventory() -> Result<Vec<String>> {
    touched_loc::density_inventory()
}

/// Returns diagnostics for one authorization record and captured PR state.
#[must_use]
pub fn merge_authorization_diagnostics(authorization: &str, pr_state: &str) -> Vec<String> {
    merge_authorization::check(authorization, pr_state)
}

/// Resolves one typed child-routing request from the packaged policy data.
pub fn resolve_child_routing(plugin_root: &Path, request: &str) -> Result<serde_json::Value> {
    routing_policy::resolve(plugin_root, request)
}

/// Resolves typed work boundaries into their TDD and proportional-proof duties.
///
/// # Errors
///
/// Returns an error for unreadable, malformed, or incomplete policy or request data.
pub fn resolve_tdd_classification(plugin_root: &Path, request: &str) -> Result<serde_json::Value> {
    tdd_classification::resolve(plugin_root, request)
}

pub use review::{
    build_review_pr_state, check_review_economics, check_review_packet, resolve_review_profile,
};

fn require_string(value: Option<&serde_json::Value>, field: &str, path: &Path) -> Result<String> {
    value
        .and_then(serde_json::Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} {field} must be a non-empty string",
                crate::paths::display_relative(path)
            )
        })
}

fn load_json(path: &Path) -> Result<serde_json::Value> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        anyhow::anyhow!(
            "missing required file: {}: {error}",
            crate::paths::display_relative(path)
        )
    })?;
    serde_json::from_str(&text).map_err(|error| {
        anyhow::anyhow!(
            "invalid JSON in {}: {error}",
            crate::paths::display_relative(path)
        )
    })
}

fn load_toml(path: &Path) -> Result<toml::Value> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        anyhow::anyhow!(
            "missing TOML file: {}: {error}",
            crate::paths::display_relative(path)
        )
    })?;
    toml::from_str(&text).map_err(|error| {
        anyhow::anyhow!(
            "invalid TOML in {}: {error}",
            crate::paths::display_relative(path)
        )
    })
}

fn manifest_path(plugin_root: &Path) -> std::path::PathBuf {
    plugin_root.join(".codex-plugin/plugin.json")
}
