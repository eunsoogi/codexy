use std::path::Path;

use anyhow::Result;

use super::review_control;

/// Resolves one typed review profile from the packaged review-control policy.
///
/// # Errors
///
/// Returns an error for malformed profile requests or policy drift.
pub fn resolve_review_profile(plugin_root: &Path, request: &str) -> Result<serde_json::Value> {
    review_control::resolve_profile(plugin_root, request)
}

/// Validates one typed review packet against the packaged bounded-review policy.
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

/// Validates one typed review-economics report against parity and profile budgets.
///
/// # Errors
///
/// Returns an error for corpus drift, missing seeded-defect parity, or an exceeded profile budget.
pub fn check_review_economics(
    plugin_root: &Path,
    repository_root: &Path,
    economics: &str,
) -> Result<()> {
    review_control::check_economics(plugin_root, repository_root, economics)
}

/// Builds canonical PR state after validating the complete typed review-control history.
///
/// # Errors
///
/// Returns an error for malformed state, stale evidence, or an invalid bounded review ledger.
pub fn build_review_pr_state(
    plugin_root: &Path,
    base: &str,
    control: &str,
) -> Result<serde_json::Value> {
    review_control::build_pr_state(plugin_root, base, control)
}
