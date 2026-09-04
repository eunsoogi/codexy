use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use super::{
    Mode, child_goal_blocked_audit, child_goal_reporting, child_lane_ownership,
    child_lifecycle_events, child_terminal_handoff, completion_handoff, conventional_commit,
    getcodexy_component_contract, github_labels, hooks, lsp, manifest, mcp, merge_authorization,
    merge_message, review_control, roles, roles_yaml, routing_policy, runtime, tdd_classification,
    touched_loc, workflow_profile_evidence, workflow_profiles,
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
            if is_devtools(plugin_root) {
                all.extend(lsp::check(plugin_root));
                all.extend(mcp::check(plugin_root));
                all.extend(roles_yaml::check(plugin_root));
                return all;
            }
            all.extend(hooks::check(plugin_root));
            all.extend(roles::check(plugin_root));
            all.extend(routing_policy::check(plugin_root));
            all.extend(tdd_classification::check(plugin_root));
            all.extend(review_control::check(plugin_root));
            all.extend(workflow_profiles::check(plugin_root));
            all.extend(getcodexy_component_contract::check(plugin_root));
            let devtools = devtools_root(plugin_root);
            if devtools.is_dir() {
                all.extend(manifest::check(&devtools));
                all.extend(lsp::check(&devtools));
                all.extend(mcp::check(&devtools));
                all.extend(roles_yaml::check(&devtools));
            }
            all
        }
        Mode::Lsp => lsp::check(&tooling_root(plugin_root)),
        Mode::RustLspReadiness => lsp::check_rust_readiness(&tooling_root(plugin_root)),
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
        Mode::CompletionHandoff { handoff, pr_state } => {
            let mut errors = completion_handoff::check(plugin_root, &handoff, &pr_state);
            errors.extend(github_labels::check_completion_handoff(&handoff, &pr_state));
            errors
        }
        Mode::Mcp => mcp::check(&tooling_root(plugin_root)),
        Mode::Hooks => hooks::check(plugin_root),
        Mode::Roles => roles::check(plugin_root),
        Mode::RuntimeArtifacts => runtime::check_artifacts(plugin_root),
        Mode::ChildLaneOwnership { evidence } => {
            let mut errors = child_lane_ownership::check(&evidence);
            errors.extend(child_goal_reporting::check(&evidence));
            errors.extend(workflow_profiles::check_evidence(plugin_root, &evidence));
            errors.extend(review_lifecycle_errors(plugin_root, &evidence));
            errors.extend(child_terminal_errors(&evidence));
            errors.extend(child_goal_blocked_audit::check(plugin_root, &evidence));
            errors
        }
        Mode::TouchedLoc { base_ref } => touched_loc::check(&base_ref),
    }
}

fn review_lifecycle_errors(plugin_root: &Path, evidence: &str) -> Vec<String> {
    workflow_profile_evidence::current_active_lines(evidence)
        .into_iter()
        .filter(|line| {
            serde_json::from_str::<serde_json::Value>(line).is_ok_and(|record| {
                record.get("reviewed_head").is_some()
                    && record.get("profile").is_some()
                    && record.get("reviewer").is_some()
            })
        })
        .filter(|line| !review_control::is_lifecycle_terminal(plugin_root, line))
        .map(|_| "review lifecycle evidence must contain direct terminal fields".to_owned())
        .collect()
}

fn child_terminal_errors(evidence: &str) -> Vec<String> {
    let active = child_lifecycle_events::active_lines(evidence);
    let lines = active
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    let source = lines
        .iter()
        .find_map(|line| line.strip_prefix("source thread id: "))
        .filter(|value| !value.is_empty());
    let mut errors = child_terminal_handoff::check(&lines, source);
    if lines.iter().any(|line| {
        line.strip_prefix("parent route: ")
            .and_then(|route| route.split([';', ',', ' ']).next())
            .is_some_and(child_terminal_handoff::is_local_task_target)
    }) {
        errors.push("child goal reporting must not use local agents /root routing".into());
    }
    errors
}

/// Returns the LSP file extensions covered by Codexy validation metadata.
///
/// # Errors
///
/// Returns an error when the packaged LSP metadata is unreadable or malformed.
pub fn covered_extensions(plugin_root: &Path) -> Result<Vec<String>> {
    lsp::covered_extensions(&public_devtools_root(plugin_root))
}

fn is_devtools(plugin_root: &Path) -> bool {
    std::fs::read_to_string(plugin_root.join(".codex-plugin/plugin.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|manifest| {
            manifest
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .as_deref()
        == Some("codexy-devtools")
}

fn devtools_root(plugin_root: &Path) -> PathBuf {
    plugin_root.parent().map_or_else(
        || PathBuf::from("plugins/codexy-devtools"),
        |parent| parent.join("codexy-devtools"),
    )
}

fn public_devtools_root(plugin_root: &Path) -> PathBuf {
    if plugin_root.file_name().is_some_and(|name| name == "codexy") {
        return plugin_root.parent().map_or_else(
            || PathBuf::from("plugins/codexy-devtools"),
            |parent| parent.join("codexy-devtools"),
        );
    }
    plugin_root.to_path_buf()
}

fn tooling_root(plugin_root: &Path) -> PathBuf {
    if !is_devtools(plugin_root) && !plugin_root.join(".codex/lsp-client.json").is_file() {
        return devtools_root(plugin_root);
    }
    plugin_root.to_path_buf()
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
