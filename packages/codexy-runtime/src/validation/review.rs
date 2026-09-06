use std::path::Path;

use anyhow::Result;

use super::review_control;

/// Resolves one typed review profile from the packaged profile policy.
///
/// # Errors
///
/// Returns an error for malformed profile requests or policy drift.
pub fn resolve_review_profile(plugin_root: &Path, request: &str) -> Result<serde_json::Value> {
    review_control::resolve_profile(plugin_root, request)
}

/// Keeps the legacy packet entry point non-blocking.
///
/// # Errors
///
/// Returns an error for malformed packets, stale repository evidence, or invalid durable review transitions.
pub fn check_review_packet(
    plugin_root: &Path,
    repository_root: &Path,
    ledger_path: &Path,
    packet: &str,
) -> Result<()> {
    review_control::check_packet(plugin_root, repository_root, ledger_path, packet)
}

/// Keeps the legacy measurement entry point non-blocking.
///
/// # Errors
///
/// Returns an error for malformed unavailable state or whenever no independent
/// Codex task/tool authority is exposed for an observed report.
pub fn check_review_economics(
    plugin_root: &Path,
    repository_root: &Path,
    economics: &str,
) -> Result<()> {
    review_control::check_economics(plugin_root, repository_root, economics)
}

/// Builds PR state after validating direct current-head review state against
/// authenticated current and previous PR snapshots.
///
/// # Errors
///
/// Returns an error for malformed state or stale direct review evidence.
pub fn build_review_pr_state(
    plugin_root: &Path,
    repository_root: &Path,
    base: &str,
    control: &str,
    previous: &str,
) -> Result<serde_json::Value> {
    review_control::build_pr_state(plugin_root, repository_root, base, control, previous)
}

/// Imports complete, pre-PR reviewer history into one authenticated current PR snapshot.
///
/// # Errors
///
/// Returns an error for incomplete source evidence, invalid identities, or missing Git ancestry.
pub fn import_pre_pr_review_history(
    plugin_root: &Path,
    repository_root: &Path,
    current: &str,
    envelope: &str,
) -> Result<serde_json::Value> {
    review_control::import_pre_pr_history(plugin_root, repository_root, current, envelope)
}

/// Returns direct review state from the compatibility producer entry point.
///
/// # Errors
///
/// Returns an error when the input is malformed or not an object.
pub fn produce_review_control(
    plugin_root: &Path,
    repository_root: &Path,
    request: &str,
) -> Result<serde_json::Value> {
    review_control::produce(plugin_root, repository_root, request)
}

/// Checks whether one authenticated mixed-finding post-cap review may run.
///
/// # Errors
///
/// Returns an error for stale snapshots, forged inputs, incomplete sources, or
/// a predecessor that is not the exact two-event delta BLOCK state.
pub fn check_next_review_eligibility(
    plugin_root: &Path,
    repository_root: &Path,
    current: &str,
    previous: &str,
    request: &str,
) -> Result<serde_json::Value> {
    review_control::check_next_review_eligibility(
        plugin_root,
        repository_root,
        current,
        previous,
        request,
    )
}
