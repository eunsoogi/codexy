//! Pure boundary for recovering review history from complete native host reads.
//!
//! This module deliberately has no host, GitHub, credential, or durable-state
//! access. Callers must capture the supported host records first and bind a
//! separately captured PR snapshot only through bind_current_pr_snapshot.

use std::path::Path;

#[path = "native_history/projection.rs"]
mod projection;
#[path = "native_history/recovery.rs"]
mod recovery;
#[path = "native_history/source.rs"]
mod source;

pub(crate) const REQUEST_SCHEMA: &str = "codexy.review-control-native-history-request.v1";
pub(crate) const RECEIPT_SCHEMA: &str = "codexy.review-control-native-history.v1";

/// Normalize one complete owner/reviewer source request without admitting it.
pub(crate) fn normalize_native_history(
    input: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let captured = source::capture(input)?;
    projection::project(captured)
}

/// Bind a fresh, separately captured PR snapshot to an existing receipt.
///
/// The result remains a non-admitted projection. In particular, a caller
/// supplied authenticated field is not treated as credential proof.
pub(crate) fn bind_current_pr_snapshot(
    receipt: &serde_json::Value,
    snapshot: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    projection::bind(receipt, snapshot)
}

pub(crate) fn recover_text(
    plugin_root: &Path,
    current_text: &str,
    input_text: &str,
) -> Result<serde_json::Value, String> {
    recovery::recover_text(plugin_root, current_text, input_text)
}

pub(crate) fn recover_native_history(
    plugin_root: &Path,
    current_text: &str,
    input_text: &str,
) -> anyhow::Result<serde_json::Value> {
    recover_text(plugin_root, current_text, input_text).map_err(anyhow::Error::msg)
}
