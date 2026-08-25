
pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "validator_completion_handoff_waiting/event_matrix.rs"]
mod event_matrix;
#[path = "validator_completion_handoff_waiting/contraction_polarity.rs"]
mod contraction_polarity;

const OPEN_PR_STATE: &str =
    r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#;

#[test]
fn validator_rejects_non_blocking_waits_described_as_blocked() -> TestResult {
    for handoff in [
        "Goal blocked because child-thread work is still pending.",
        "Blocked: child thread verification is still pending.",
        "Blocker: queued worktree setup has not completed yet.",
        "Blocked on asynchronous tool completion.",
        "Blocked: async tool has not returned yet.",
        "Blocked while waiting for parent authorization.",
        "Blocked while waiting for dependency integration.",
        "Blocked while waiting for a resource slot.",
        "Blocked while waiting for a Sentinel result.",
        "Blocked while waiting for CI completion.",
        "Blocked while waiting for connector review.",
        "Blocked while reviewer feedback is pending.",
        "Blocked while waiting for reviewer feedback; no actionable feedback has arrived.",
        "Blocked after repeated true impasse: cannot make meaningful progress without maintainer input.",
        "Blocked after repeated true impasse because an external state change is required.",
    ] {
        let output = validate(handoff)?;
        assert!(
            !output.status.success(),
            "handoff unexpectedly passed: {handoff}"
        );
        assert!(stderr(&output).contains("waiting state"));
    }
    Ok(())
}

#[test]
fn validator_rejects_negated_resolved_and_pending_review_waits() -> TestResult {
    for handoff in [
        "Blocked while reviewer reports no requested changes are pending.",
        "Blocked while requested changes are resolved and reviewer confirmation is pending.",
        "Blocked while feedback from the maintainer is pending.",
        "Blocked while a security review is pending.",
    ] {
        let output = validate(handoff)?;
        assert!(
            !output.status.success(),
            "review wait unexpectedly passed: {handoff}"
        );
        assert!(
            stderr(&output).contains("waiting state"),
            "missing waiting diagnostic for {handoff}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_uncertainty_and_token_pressure_as_blockers() -> TestResult {
    for handoff in [
        "Blocked because the implementation remains uncertain.",
        "Blocked because token pressure is high.",
    ] {
        let output = validate(handoff)?;
        assert!(
            !output.status.success(),
            "operational pressure unexpectedly passed: {handoff}"
        );
        assert!(
            stderr(&output).contains("waiting state"),
            "missing waiting diagnostic for {handoff}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_prioritizes_actionable_review_over_operational_context() -> TestResult {
    for handoff in [
        "Blocked: requested changes remain unresolved while work is incomplete.",
        "Blocked: review feedback is not resolved while work is incomplete.",
        "Blocked: review feedback is not yet considered fully resolved.",
    ] {
        let output = validate(handoff)?;
        assert!(
            output.status.success(),
            "actionable mixed context was rejected: {handoff}\n{}",
            stderr(&output)
        );
    }

    for handoff in [
        "Blocked: requested changes are not unresolved.",
        "Blocked: review feedback is resolved while work is incomplete.",
        "Blocked: no review feedback remains unresolved while work is incomplete.",
        "Blocked: no requested changes remain unresolved while work is incomplete.",
        "Blocked: neither security review nor code review remains unresolved.",
    ] {
        let output = validate(handoff)?;
        assert!(
            !output.status.success(),
            "nonterminal review context unexpectedly passed: {handoff}"
        );
        assert!(
            stderr(&output).contains("waiting state"),
            "missing waiting diagnostic for {handoff}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_binds_review_state_to_its_predicate() -> TestResult {
    for handoff in [
        "Blocked: review feedback is resolved while work remains unresolved and incomplete.",
        "Blocked: work remains open while review feedback is resolved.",
    ] {
        let output = validate(handoff)?;
        assert!(
            !output.status.success(),
            "operational state was incorrectly attached to review feedback: {handoff}"
        );
        assert!(stderr(&output).contains("waiting state"));
    }

    for handoff in [
        "Blocked: review feedback remains unresolved while work is incomplete.",
        "Blocked: review feedback remains open while work is incomplete.",
        "Blocked: no wait but review feedback remains unresolved.",
    ] {
        let output = validate(handoff)?;
        assert!(
            output.status.success(),
            "review-owned actionable state was rejected: {handoff}\n{}",
            stderr(&output)
        );
    }

    for handoff in ["Blocked: no wait but review feedback is resolved."] {
        let output = validate(handoff)?;
        assert!(
            !output.status.success(),
            "negation crossed into the review predicate: {handoff}"
        );
        assert!(stderr(&output).contains("waiting state"));
    }
    Ok(())
}

#[test]
fn validator_classifies_bounded_wait_subject_state_events() -> TestResult {
    let grammar = [
        ("Blocked: CI queued.", false),
        ("Blocked: CI: queued.", false),
        ("Blocked: Sentinel running.", false),
        ("Blocked: reviewer idle.", false),
        ("Blocked: security review queued.", false),
        ("Blocked: resource unavailable.", false),
        ("Blocked: background tool running.", false),
        ("Blocked: review feedback: pending.", false),
        (
            "Blocked: review feedback is pending while implementation remains open.",
            false,
        ),
        (
            "Blocked: implementation remains open but review feedback is pending.",
            false,
        ),
        (
            "Blocked: review feedback remains unresolved while implementation remains open.",
            true,
        ),
    ];
    for (handoff, actionable) in grammar {
        let output = validate(handoff)?;
        assert_eq!(
            output.status.success(),
            actionable,
            "incorrect bounded event disposition for {handoff}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_real_blockers() -> TestResult {
    for handoff in [
        "Blocked: review feedback requested changes remain unresolved.",
        "Blocked: CONNECTOR_REPAIR_CURRENT_HEAD_NON_READY; connector repair changed the current head, so current-head selected-review proof is the actionable repair blocker; 2/3 ledger retained; no fourth profile-selected review; no fabricated verdict; no another connector review.",
        "Blocked: requested changes are not resolved.",
        "Blocked: required status checks are failing.",
        "Blocked: required checks failed during a hard investigation.",
        "Blocked: child thread omitted required goal tool evidence.",
        "Blocked: worktree setup failed with an invalid reference.",
        "Blocked: async tool failed authentication.",
    ] {
        let output = validate(handoff)?;
        assert!(
            output.status.success(),
            "real blocker was rejected: {handoff}\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_mixed_connector_repair_dispositions() -> TestResult {
    for suffix in [
        "request a fourth profile-selected review.",
        "PARENT_DECISION.",
        "fabricated PASS.",
        "request another connector review.",
    ] {
        let output = validate(&format!("Blocked: CONNECTOR_REPAIR_CURRENT_HEAD_NON_READY; {suffix}"))?;
        assert!(!output.status.success() && stderr(&output).contains("connector-repair disposition"));
    }
    Ok(())
}

#[test]
fn validator_preserves_true_impasse() -> TestResult {
    let output = validate(
        "Blocked by an unanswered user decision that materially changes the result; no safe default and no in-scope action exist.",
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

pub(super) fn validate(handoff: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, OPEN_PR_STATE)?;
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}

pub(super) fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
