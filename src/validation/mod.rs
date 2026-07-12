mod agent_model_contract;
mod agent_registration;
mod agent_registration_catalog;
mod child_handoff_readiness;
mod child_handoff_readiness_claims;
mod child_handoff_readiness_heads;
mod child_handoff_readiness_status;
mod child_handoff_readiness_text;
#[path = "child_lane_active_threads_module.rs"]
mod child_lane_active_threads;
mod child_lane_classification_boundaries;
mod child_lane_classification_setup;
mod child_lane_classification_setup_context;
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
mod child_lane_thread_tool_handler_no_route;
mod child_lane_thread_tool_handler_route_value;
mod child_lane_thread_tool_handler_scope;
mod child_lane_thread_tool_handlers;
mod child_lane_thread_tools;
mod codex_review_fresh_request;
mod codex_review_fresh_request_action;
mod codex_review_fresh_request_text;
mod codex_review_handoff;
mod codex_review_handoff_actionable;
mod codex_review_handoff_events;
mod codex_review_handoff_readiness;
mod completion_handoff;
mod completion_handoff_compaction;
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
mod github_labels;
mod hooks;
mod instruction_policy;
mod instruction_policy_match;
mod instruction_policy_purpose;
mod instruction_policy_text;
mod lsp;
mod manifest;
mod mcp;
mod mcp_runtime;
mod merge_message;
mod orchestration_routing;
mod orchestration_routing_semantics;
mod prompt_yaml;
mod release_publish_contract;
mod review_thread_evidence;
mod review_thread_readiness;
mod review_thread_resolution;
mod review_thread_waiting;
mod review_thread_waiting_phrases;
mod review_thread_waiting_refs;
mod roles;
mod roles_yaml;
mod runtime;
mod sentinel_handoff;
mod sentinel_handoff_evidence;
mod sentinel_handoff_reviewer;
mod touched_loc;

use std::path::Path;

use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub enum Mode {
    All,
    Lsp,
    RustLspReadiness,
    MergeMessage {
        expected_issue: Option<u64>,
        expected_pr: Option<u64>,
        message: String,
    },
    PrTitle {
        title: String,
    },
    IssueTitle {
        title: String,
    },
    CompletionHandoff {
        handoff: String,
        pr_state: String,
    },
    Mcp,
    Hooks,
    Roles,
    RuntimeArtifacts,
    ChildLaneOwnership {
        evidence: String,
    },
    TouchedLoc {
        base_ref: String,
    },
}

/// Runs plugin contract validation for the selected mode.
///
/// # Errors
///
/// Returns an error when any selected validation surface reports contract
/// failures.
pub fn run(plugin_root: &Path, mode: Mode) -> Result<()> {
    let errors = match mode {
        Mode::All => {
            let mut all = Vec::new();
            all.extend(manifest::check(plugin_root));
            all.extend(hooks::check(plugin_root));
            all.extend(lsp::check(plugin_root));
            all.extend(mcp::check(plugin_root));
            all.extend(roles::check(plugin_root));
            all.extend(instruction_policy::check(plugin_root));
            all.extend(orchestration_routing::check(plugin_root));
            all
        }
        Mode::Lsp => lsp::check(plugin_root),
        Mode::RustLspReadiness => lsp::check_rust_readiness(plugin_root),
        Mode::MergeMessage {
            expected_issue,
            expected_pr,
            message,
        } => merge_message::check(expected_issue, expected_pr, &message),
        Mode::PrTitle { title } => conventional_commit::check_pr_title(&title),
        Mode::IssueTitle { title } => conventional_commit::check_issue_title(&title),
        Mode::CompletionHandoff { handoff, pr_state } => {
            let mut errors = completion_handoff::check(&handoff, &pr_state);
            errors.extend(github_labels::check_completion_handoff(&handoff, &pr_state));
            errors
        }
        Mode::Mcp => mcp::check(plugin_root),
        Mode::Hooks => hooks::check(plugin_root),
        Mode::Roles => {
            let mut errors = roles::check(plugin_root);
            errors.extend(instruction_policy::check_roles(plugin_root));
            errors
        }
        Mode::RuntimeArtifacts => runtime::check_artifacts(plugin_root),
        Mode::ChildLaneOwnership { evidence } => child_lane_ownership::check(&evidence),
        Mode::TouchedLoc { base_ref } => touched_loc::check(&base_ref),
    };
    if errors.is_empty() {
        Ok(())
    } else {
        for error in &errors {
            eprintln!("error: {error}");
        }
        bail!("plugin validation failed with {} error(s)", errors.len())
    }
}

/// Returns the LSP file extensions covered by Codexy validation metadata.
///
/// # Errors
///
/// Returns an error when LSP configuration files cannot be read or parsed.
pub fn covered_extensions(plugin_root: &Path) -> Result<Vec<String>> {
    lsp::covered_extensions(plugin_root)
}

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

fn json_array_strings(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .map(serde_json::Value::as_str)
                .collect::<Option<Vec<_>>>()
                .map(|strings| strings.into_iter().map(ToOwned::to_owned).collect())
        })
}

fn toml_array_strings(value: Option<&toml::Value>) -> Option<Vec<String>> {
    value.and_then(toml::Value::as_array).and_then(|items| {
        items
            .iter()
            .map(toml::Value::as_str)
            .collect::<Option<Vec<_>>>()
            .map(|strings| strings.into_iter().map(ToOwned::to_owned).collect())
    })
}
