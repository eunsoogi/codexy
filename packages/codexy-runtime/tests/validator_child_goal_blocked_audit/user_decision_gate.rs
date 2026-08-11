use super::{
    CLASSIFICATION, TestResult, blocked_evidence, run_validator, valid_gate, valid_pre_mutation,
};

#[path = "user_decision_gate/numeric_semantics.rs"]
mod numeric_semantics;

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

    let ordinary_domain_repetition = run_validator(&blocked_evidence(
        valid_gate()
            .replace(
                "Should the irreversible migration preserve legacy identifiers or replace them?",
                "Should the account migration preserve account identifiers or require replacement?",
            )
            .replace(
                "preserve identifiers and retain compatibility|replace identifiers and require migration",
                "preserve existing identifiers and keep existing compatibility|replace existing identifiers and require migration",
            )
            .replace(
                "the choice changes persisted identifiers and migration behavior",
                "the account migration choice changes account identifiers and compatibility",
            ),
        valid_pre_mutation(),
    ))?;
    assert!(
        ordinary_domain_repetition.status.success(),
        "ordinary repeated domain terms were rejected: {}",
        String::from_utf8_lossy(&ordinary_domain_repetition.stderr)
    );

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
        ("reviewer", "reviewer-pending", "review-event", ""),
        (
            "async-tool",
            "async-tool-pending",
            "tool-result-event",
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
        valid_gate().replace("Should the irreversible migration preserve legacy identifiers or replace them?", "?"),
        valid_gate().replace("Should the irreversible migration preserve legacy identifiers or replace them?", "x?"),
        valid_gate().replace("Should the irreversible migration preserve legacy identifiers or replace them?", "x x x x x x x x x x x x?"),
        valid_gate().replace("Should the irreversible migration preserve legacy identifiers or replace them?", "choice choice choice choice?"),
        valid_gate().replace("Should the irreversible migration preserve legacy identifiers or replace them?", "choose migration choose migration?"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "same branch|same branch"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "none|unavailable"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "x|y"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "none none none|unavailable unavailable unavailable"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "alpha alpha alpha|beta beta beta"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "alpha beta alpha beta|gamma delta gamma delta"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "alpha beta alpha|gamma delta gamma"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "preserve existing identifiers|identifiers existing preserve"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "preserve identifiers and retain compatibility|replace identifiers and require migration|"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "preserve identifiers and retain compatibility|replace identifiers and require migration|alpha beta alpha"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "preserve identifiers and retain compatibility|replace identifiers and require migration|preserve identifiers and retain compatibility"),
        valid_gate().replace("preserve identifiers and retain compatibility|replace identifiers and require migration", "preserve identifiers and retain compatibility|replace identifiers and require migration|compatibility retain identifiers preserve"),
        valid_gate().replace("material impact=the choice changes persisted identifiers and migration behavior", "material impact=unavailable"),
        valid_gate().replace("material impact=the choice changes persisted identifiers and migration behavior", "material impact=none"),
        valid_gate().replace("material impact=the choice changes persisted identifiers and migration behavior", "material impact=x"),
        valid_gate().replace("material impact=the choice changes persisted identifiers and migration behavior", "material impact=none none none none"),
        valid_gate().replace("material impact=the choice changes persisted identifiers and migration behavior", "material impact=impact impact impact impact"),
        valid_gate().replace("material impact=the choice changes persisted identifiers and migration behavior", "material impact=impact result impact result"),
        valid_gate().replace("material impact=the choice changes persisted identifiers and migration behavior", "material impact=alpha beta gamma alpha"),
        valid_gate().replace("safe default=unavailable", "safe default=preserve identifiers"),
        valid_gate().replace("in-scope action=unavailable", "in-scope action=inspect repository"),
        valid_gate().replace("blocker class=user-decision", "blocker class=parent-authorization"),
    ] {
        let output = run_validator(&blocked_evidence(&invalid, valid_pre_mutation()))?;
        assert!(
            !output.status.success(),
            "malformed user-decision gate passed: {invalid}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_short_token_padding_in_a_question() -> TestResult {
    assert_short_token_padding_rejected(valid_gate().replace(
        "Should the irreversible migration preserve legacy identifiers or replace them?",
        "use use use use choose migration?",
    ))
}

#[test]
fn validator_rejects_short_token_padding_in_every_branch() -> TestResult {
    for branches in [
        "use use use use choose migration|replace identifiers and require migration",
        "preserve identifiers and retain compatibility|use use use use choose migration",
    ] {
        assert_short_token_padding_rejected(valid_gate().replace(
            "preserve identifiers and retain compatibility|replace identifiers and require migration",
            branches,
        ))?;
    }
    Ok(())
}

#[test]
fn validator_rejects_short_token_padding_in_material_impact() -> TestResult {
    assert_short_token_padding_rejected(valid_gate().replace(
        "material impact=the choice changes persisted identifiers and migration behavior",
        "material impact=use use use use choose migration",
    ))
}

#[test]
fn validator_accepts_terminal_handoff_material_impact() -> TestResult {
    let output = run_validator(&blocked_evidence(
        valid_gate().replace(
            "material impact=the choice changes persisted identifiers and migration behavior",
            "material impact=the choice changes the destination and access boundary",
        ),
        valid_pre_mutation(),
    ))?;
    assert!(
        output.status.success(),
        "terminal-handoff material impact was rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_short_token_padding_rejected(gate: String) -> TestResult {
    let output = run_validator(&blocked_evidence(gate, valid_pre_mutation()))?;
    assert!(
        !output.status.success(),
        "short-token repetition passed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
