use serde_json::json;

use crate::support::TestResult;

use super::{check_packet, packet};

#[test]
fn terminal_transitions_preserve_the_reviewed_scope() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let ledger = temp.path().join("ledger.json");
    let mut full = packet("e-full", "full");
    full["findings"] = json!([]);
    full["resolution"] = json!({"repaired_finding_ids":[],"changed_boundaries":[]});
    full["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    assert!(check_packet(fixture.root(), &ledger, &full)?.status.success());

    let mut passed = full;
    passed["event_id"] = json!("e-passed");
    passed["predecessor_event_id"] = json!("e-full");
    passed["state"] = json!("passed");
    passed["direct_boundaries"] = json!(["other"]);
    assert!(
        !check_packet(fixture.root(), &ledger, &passed)?.status.success(),
        "a terminal event must not replace the reviewed boundary scope"
    );
    Ok(())
}
