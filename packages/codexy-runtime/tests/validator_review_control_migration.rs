use serde_json::json;

use crate::support::TestResult;

#[path = "support/review_control_migration_continued.rs"]
mod continued;

#[path = "support/review_control_migration_fixtures.rs"]
mod fixtures;

#[path = "support/review_control_migration_runner.rs"]
mod runner;

#[test]
fn in_flight_model_changes_preserve_strict_and_standard_histories() -> TestResult {
    for profile in ["strict", "standard"] {
        let previous = fixtures::legacy_control(profile, 725, runner::HEAD_OID);
        let current = fixtures::migrated_control(
            profile,
            725,
            runner::HEAD_OID,
            runner::MIGRATED_HEAD_OID,
        );
        let (result, state) = runner::run_transition(&previous, &current)?;
        assert!(
            result.status.success(),
            "{profile} predecessor history must survive the fixed model upgrade: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let state = state.expect("successful transition must write a PR state");
        assert_eq!(
            state["reviewControl"]["reviewer_migration"]["schema"],
            "codexy.review-control-migration.v1"
        );
        assert_eq!(
            state["reviewControl"]["reviewer_migration"]["history_boundary"],
            1
        );
    }
    Ok(())
}

#[test]
fn in_flight_model_changes_reject_wrong_current_reviewer_contracts() -> TestResult {
    for profile in ["strict", "standard"] {
        let previous = fixtures::legacy_control(profile, 725, runner::HEAD_OID);

        let mut retained_legacy = fixtures::migrated_control(
            profile,
            725,
            runner::HEAD_OID,
            runner::MIGRATED_HEAD_OID,
        );
        retained_legacy["reviewer"] = previous["reviewer"].clone();
        let (result, _) = runner::run_transition(&previous, &retained_legacy)?;
        assert!(!result.status.success());

        let mut arbitrary = fixtures::migrated_control(
            profile,
            725,
            runner::HEAD_OID,
            runner::MIGRATED_HEAD_OID,
        );
        arbitrary["reviewer"]["model"] = json!("gpt-9.9-unapproved");
        arbitrary["terminal_review_history"][1]["reviewer"]["model"] =
            json!("gpt-9.9-unapproved");
        let (result, _) = runner::run_transition(&previous, &arbitrary)?;
        assert!(!result.status.success());

        let mut forged_marker = fixtures::migrated_control(
            profile,
            725,
            runner::HEAD_OID,
            runner::MIGRATED_HEAD_OID,
        );
        forged_marker["reviewer_migration"] = json!({
            "schema": "codexy.review-control-migration.v1",
            "from": previous["reviewer"].clone(),
            "to": {"name": "codexy-sentinel", "model": "gpt-5.6-terra", "reasoning_effort": "max"},
            "history_boundary": 1
        });
        let (result, _) = runner::run_transition(&previous, &forged_marker)?;
        assert!(!result.status.success());
    }
    Ok(())
}

#[test]
fn review_control_producer_migrates_authenticated_predecessor_history() -> TestResult {
    for profile in ["strict", "standard"] {
        let previous = fixtures::legacy_control(profile, 725, runner::HEAD_OID);
        let current = fixtures::migrated_control(
            profile,
            725,
            runner::HEAD_OID,
            runner::MIGRATED_HEAD_OID,
        );
        let (result, state) = runner::run_producer(&previous, &current)?;
        assert!(
            result.status.success(),
            "{profile} producer must normalize authenticated predecessor history: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            state.expect("successful producer must write control")["reviewer_migration"][
                "history_boundary"
            ],
            1
        );
    }
    Ok(())
}

#[test]
fn continued_transition_preserves_the_initial_migration_marker() -> TestResult {
    for profile in ["strict", "standard"] {
        let state = continued::run(profile)?;
        assert_eq!(state["reviewControl"]["terminal_review_count"], 3);
        assert_eq!(
            state["reviewControl"]["reviewer_migration"]["history_boundary"],
            1
        );
        assert_eq!(
            state["reviewControl"]["terminal_review_history"][0]["reviewer"]["model"],
            if profile == "strict" {
                "gpt-5.6-sol"
            } else {
                "gpt-5.6-terra"
            }
        );
        assert_eq!(
            state["reviewControl"]["terminal_review_history"][2]["reviewer"]["model"],
            if profile == "strict" {
                "gpt-6-astra"
            } else {
                "gpt-5.6-sol"
            }
        );
    }
    Ok(())
}
