use serde_json::json;

use crate::support::TestResult;

use super::{check_packet, packet, set_profile};

#[test]
fn full_packets_consolidate_duplicate_blocker_classes_for_every_profile() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;

    for (profile, reviewer) in [
        (
            "standard",
            json!({"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"}),
        ),
        (
            "strict",
            json!({"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"}),
        ),
    ] {
        let mut duplicate = packet(&format!("{profile}-duplicate-class"), "full");
        set_profile(&mut duplicate, profile, reviewer);
        let mut repeated = duplicate["findings"][0].clone();
        repeated["id"] = json!("f-2");
        repeated["counterexample"] = json!("another spelling of the same root cause");
        duplicate["findings"]
            .as_array_mut()
            .ok_or("findings must be an array")?
            .push(repeated);
        duplicate["readiness_export"]["unresolved_blocker_ids"] = json!(["f-1", "f-2"]);
        assert!(
            !check_packet(
                fixture.root(),
                &temp.path().join(format!("{profile}-duplicate.json")),
                &duplicate,
            )?
            .status
            .success()
        );

        let mut distinct = duplicate;
        distinct["event_id"] = json!(format!("{profile}-distinct-class"));
        distinct["findings"][1]["defect_class"] = json!("compatibility");
        assert!(
            check_packet(
                fixture.root(),
                &temp.path().join(format!("{profile}-distinct.json")),
                &distinct,
            )?
            .status
            .success()
        );
    }

    Ok(())
}
