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

/// Applies the fail-closed review-economics contract to one typed report.
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
    if let Ok(request) = serde_json::from_str::<serde_json::Value>(economics) {
        if request["schema"] == "codexy.review-economics-capture-request.v1" {
            return review_control::capture_economics(
                plugin_root,
                repository_root,
                Path::new(
                    request["observer_command"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("capture observer command is missing"))?,
                ),
                Path::new(
                    request["trusted_receipt"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("capture trusted receipt is missing"))?,
                ),
                Path::new(
                    request["output"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("capture output is missing"))?,
                ),
            );
        }
    }
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
