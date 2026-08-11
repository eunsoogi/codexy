use std::path::Path;

use anyhow::{Result, bail};

use super::{
    Mode, child_goal_blocked_audit, child_goal_reporting, child_lane_ownership, completion_handoff,
    conventional_commit, getcodexy_component_contract, github_labels, hooks, issue_intake, lsp,
    manifest, mcp, merge_authorization, merge_message, roles, routing_measurement, routing_policy,
    runtime, touched_loc, workflow_profiles,
};

/// Runs plugin contract validation for the selected mode.
///
/// # Errors
///
/// Returns an error when any selected validation surface reports contract
/// failures.
#[must_use]
pub fn errors(plugin_root: &Path, mode: Mode) -> Vec<String> {
    match mode {
        Mode::All => {
            let mut all = Vec::new();
            all.extend(manifest::check(plugin_root));
            all.extend(hooks::check(plugin_root));
            all.extend(lsp::check(plugin_root));
            all.extend(mcp::check(plugin_root));
            all.extend(roles::check(plugin_root));
            all.extend(routing_policy::check(plugin_root));
            all.extend(workflow_profiles::check(plugin_root));
            all.extend(getcodexy_component_contract::check(plugin_root));
            all
        }
        Mode::Lsp => lsp::check(plugin_root),
        Mode::RustLspReadiness => lsp::check_rust_readiness(plugin_root),
        Mode::MergeMessage {
            expected_issue,
            expected_pr,
            message,
        } => merge_message::check(expected_issue, expected_pr, &message),
        Mode::MergeAuthorization {
            authorization,
            pr_state,
        } => merge_authorization::check(&authorization, &pr_state),
        Mode::PrTitle { title } => conventional_commit::check_pr_title(&title),
        Mode::IssueTitle { title } => conventional_commit::check_issue_title(&title),
        Mode::PrLabels { pr_state } => github_labels::check_pr_labels(&pr_state),
        Mode::IssueIntake { receipt } => issue_intake::check(&receipt),
        Mode::CompletionHandoff { handoff, pr_state } => {
            let mut errors = completion_handoff::check(&handoff, &pr_state);
            errors.extend(github_labels::check_completion_handoff(&handoff, &pr_state));
            errors
        }
        Mode::RoutingMeasurement { corpus, results } => {
            routing_measurement::diagnostics(plugin_root, &corpus, &results)
        }
        Mode::Mcp => mcp::check(plugin_root),
        Mode::Hooks => hooks::check(plugin_root),
        Mode::Roles => roles::check(plugin_root),
        Mode::RuntimeArtifacts => runtime::check_artifacts(plugin_root),
        Mode::ChildLaneOwnership { evidence } => {
            let mut errors = child_lane_ownership::check(&evidence);
            errors.extend(workflow_profiles::check_evidence(plugin_root, &evidence));
            errors.extend(child_goal_reporting::check(&evidence));
            errors.extend(child_goal_blocked_audit::check(&evidence));
            errors
        }
        Mode::TouchedLoc { base_ref } => touched_loc::check(&base_ref),
    }
}

/// Runs plugin contract validation for the selected mode.
///
/// # Errors
///
/// Returns an error when any selected validation surface reports contract
/// failures.
pub fn run(plugin_root: &Path, mode: Mode) -> Result<()> {
    let errors = errors(plugin_root, mode);
    if errors.is_empty() {
        Ok(())
    } else {
        for error in &errors {
            eprintln!("error: {error}");
        }
        bail!("plugin validation failed with {} error(s)", errors.len())
    }
}
