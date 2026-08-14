use serde_json::json;

use crate::support::TestResult;

use super::{check_packet, packet, set_profile};

#[test]
fn packet_blocks_owned_defects_and_demotes_unowned_parser_scope() -> TestResult {
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
        let mut owned = packet(&format!("{profile}-owned"), "full");
        set_profile(&mut owned, profile, reviewer);
        assert!(
            check_packet(
                fixture.root(),
                &temp.path().join(format!("{profile}-owned.json")),
                &owned,
            )?
            .status
            .success()
        );

        let mut universal = owned.clone();
        universal["event_id"] = json!(format!("{profile}-universal"));
        universal["findings"][0]["criterion_id"] = json!(null);
        universal["findings"][0]["owned_invariant"] = json!(null);
        assert!(
            !check_packet(
                fixture.root(),
                &temp.path().join(format!("{profile}-universal.json")),
                &universal,
            )?
            .status
            .success()
        );

        let mut follow_up = universal;
        follow_up["event_id"] = json!(format!("{profile}-follow-up"));
        follow_up["findings"][0]["disposition"] = json!("out_of_scope_followup");
        follow_up["findings"][0]["owned_boundary"] = json!(null);
        follow_up["findings"][0]["repair_boundary"] = json!(null);
        follow_up["readiness_export"]["unresolved_blocker_ids"] = json!([]);
        let mut improper_repair = follow_up.clone();
        improper_repair["event_id"] = json!(format!("{profile}-improper-repair"));
        improper_repair["findings"][0]["resolved"] = json!(true);
        improper_repair["resolution"] = json!({
            "repaired_finding_ids":["f-1"],
            "changed_boundaries":["validator"]
        });
        assert!(
            !check_packet(
                fixture.root(),
                &temp.path().join(format!("{profile}-improper-repair.json")),
                &improper_repair,
            )?
            .status
            .success()
        );
        assert!(
            check_packet(
                fixture.root(),
                &temp.path().join(format!("{profile}-follow-up.json")),
                &follow_up,
            )?
            .status
            .success()
        );
    }
    Ok(())
}
