#[path = "../../src/validation/review_control/native_history/projection.rs"]
mod projection;
#[path = "../../src/validation/review_control/native_history/source.rs"]
mod source;

pub(crate) const REQUEST_SCHEMA: &str = "codexy.review-control-native-history-request.v1";
pub(crate) const RECEIPT_SCHEMA: &str = "codexy.review-control-native-history.v1";

pub(crate) fn normalize_native_history(
    input: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let captured = source::capture(input)?;
    projection::project(captured)
}

pub(crate) fn bind_current_pr_snapshot(
    receipt: &serde_json::Value,
    snapshot: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    projection::bind(receipt, snapshot)
}
