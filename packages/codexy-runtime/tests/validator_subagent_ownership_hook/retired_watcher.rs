// Keep retired-role admission proof separate from active specialist cases.
use super::{assert_denied, TestResult};

#[test]
fn retired_watcher_role_is_denied() -> TestResult {
    assert_denied(
        Some("codexy-watcher"),
        "Observe the assigned Worker and report through Watcher MCP.",
        "ROLE",
    )
}
