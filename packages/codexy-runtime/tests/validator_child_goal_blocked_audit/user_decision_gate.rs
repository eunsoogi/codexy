use super::{
    CLASSIFICATION, TestResult, blocked_evidence, run_validator, valid_gate, valid_pre_mutation,
};

pub(super) fn assert_boundaries() -> TestResult {
    let genuine_choice = run_validator(&blocked_evidence(
        valid_gate(),
        valid_pre_mutation(),
    ))?;
    assert!(
        genuine_choice.status.success(),
        "genuine unanswered user choice was rejected: {}",
        String::from_utf8_lossy(&genuine_choice.stderr)
    );

    let missing_information = run_validator(&blocked_evidence(
        valid_gate().replace("blocker class=user-decision", "blocker class=missing-user-information"),
        valid_pre_mutation(),
    ))?;
    assert!(missing_information.status.success());

    for (lane, producer, wake_route, prior_event) in [
        ("547-sentinel", "sentinel-running", "sentinel-event", ""),
        (
            "550-parent-authorization",
            "parent-authorization-pending",
            "parent-message",
            "Sentinel result: BLOCK\n",
        ),
        (
            "562-dependency",
            "dependency-integration-pending",
            "dependency-merge-event",
            "",
        ),
        ("current-head-ci", "ci-queued", "check-state-event", ""),
        (
            "connector-review",
            "connector-review-pending",
            "review-event",
            "",
        ),
        ("resource-slot", "resource-slot-pending", "slot-event", ""),
        (
            "alternate-evidence",
            "alternate-evidence-pending",
            "parent-message",
            "",
        ),
        ("event-idle", "event-idle-child", "parent-message", ""),
    ] {
        let waiting = run_validator(&format!(
            "{CLASSIFICATION}{prior_event}Nonterminal wait handoff: state fingerprint={lane}; producer state={producer}; wake route={wake_route}; ownership=retained; goal state=active; plan state=active; goal transition=none; return control=confirmed\n"
        ))?;
        assert!(
            waiting.status.success(),
            "nonterminal wait {lane} was rejected: {}",
            String::from_utf8_lossy(&waiting.stderr)
        );

        let legacy_impasse = format!(
            "Blocked goal audit: audit id={lane}; first monotonic ms=1000; observed monotonic ms=61000; minimum interval ms=60000; observation ids=one|two|three; state fingerprints=one|two|three; producer state={producer}; safe action=unavailable; wake route={wake_route}\n"
        );
        let output = run_validator(&blocked_evidence(
            legacy_impasse,
            valid_pre_mutation(),
        ))?;
        assert!(
            !output.status.success(),
            "nonterminal Wave 2 wait {lane} authorized blocked"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("typed unanswered user-decision gate"),
            "missing user-decision diagnostic for {lane}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    for invalid in [
        valid_gate().replace("decision owner=user", "decision owner=parent"),
        valid_gate().replace("user response=unanswered", "user response=answered"),
        valid_gate().replace("Should the irreversible migration preserve legacy identifiers or replace them?", "none"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "same branch|same branch"),
        valid_gate().replace("material impact=the choice changes persisted identifiers and migration behavior", "material impact=unavailable"),
        valid_gate().replace("safe default=unavailable", "safe default=preserve identifiers"),
        valid_gate().replace("in-scope action=unavailable", "in-scope action=inspect repository"),
        valid_gate().replace("blocker class=user-decision", "blocker class=parent-authorization"),
    ] {
        let output = run_validator(&blocked_evidence(invalid, valid_pre_mutation()))?;
        assert!(!output.status.success(), "malformed user-decision gate passed");
    }
    Ok(())
}
