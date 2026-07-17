use std::path::Path;

use anyhow::{Result, bail};

use super::{
    Mode, child_goal_reporting, child_lane_ownership, completion_handoff, conventional_commit,
    github_labels, hooks, instruction_policy, issue_intake, lsp, manifest, mcp, merge_message,
    orchestration_routing, roles, runtime, touched_loc,
};

/// Runs plugin contract validation for the selected mode.
///
/// # Errors
///
/// Returns an error when any selected validation surface reports contract
/// failures.
pub fn errors(plugin_root: &Path, mode: Mode) -> Vec<String> {
    match mode {
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
        Mode::InstructionPolicy => instruction_policy::check(plugin_root),
        Mode::OrchestrationRouting => orchestration_routing::check(plugin_root),
        Mode::Lsp => lsp::check(plugin_root),
        Mode::RustLspReadiness => lsp::check_rust_readiness(plugin_root),
        Mode::MergeMessage {
            expected_issue,
            expected_pr,
            message,
        } => merge_message::check(expected_issue, expected_pr, &message),
        Mode::PrTitle { title } => conventional_commit::check_pr_title(&title),
        Mode::IssueTitle { title } => conventional_commit::check_issue_title(&title),
        Mode::IssueIntake { receipt } => issue_intake::check(&receipt),
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
        Mode::ChildLaneOwnership { evidence } => {
            let mut errors = child_lane_ownership::check(&evidence);
            errors.extend(child_goal_reporting::check(&evidence));
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
