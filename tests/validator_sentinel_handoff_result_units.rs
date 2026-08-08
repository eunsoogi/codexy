use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const HEAD: &str = "32b03a210b3defb2d29dd352283ea2488e60d893";

#[test]
fn validator_keeps_repeated_timeouts_nonterminal_until_same_reviewer_terminal_result() -> TestResult
{
    let prior = "Previous Packaged Sentinel Turing: UNOBSERVABLE after terminal tool failure on a prior run";
    let pending = handoff(
        &format!(
            "{prior}, and policy requires retaining Turing without messaging, interrupting, replacing, or duplicating the reviewer, but Sentinel Turing timed out after bounded wait on current head {HEAD}, and Sentinel Turing timed out after bounded wait with no event, and Sentinel Turing timed out after bounded wait with no event"
        ),
        true,
    )?;
    assert!(
        !pending.status.success(),
        "repeated bounded no-event observations must remain nonterminal"
    );
    assert!(
        String::from_utf8_lossy(&pending.stderr).contains("pending"),
        "unexpected pending result: {}",
        String::from_utf8_lossy(&pending.stderr)
    );

    for terminal in [
        format!(
            "{prior}, and policy requires retaining Turing without messaging, interrupting, replacing, or duplicating the reviewer, but Sentinel Turing timed out after bounded wait on current head {HEAD}, and Sentinel Turing timed out after bounded wait with no event, and Sentinel Turing timed out after bounded wait with no event, and Packaged Sentinel Turing: PASS on current head {HEAD}"
        ),
        format!(
            "{prior}, and policy requires retaining Turing without messaging, interrupting, replacing, or duplicating the reviewer, but Sentinel Turing timed out after bounded wait on current head {HEAD}, and Sentinel Turing timed out after bounded wait with no event, and Sentinel Turing timed out after bounded wait with no event, and Packaged Sentinel Turing: BLOCK on current head {HEAD}"
        ),
    ] {
        let output = handoff(&terminal, terminal.contains("BLOCK"))?;
        assert!(
            output.status.success(),
            "natural same-reviewer terminal result: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_replacement_or_duplication_during_same_reviewer_wait() -> TestResult {
    let prior = "Previous Packaged Sentinel Turing: UNOBSERVABLE after terminal tool failure on a prior run";
    for history in ["", prior] {
        for lifecycle in [
            format!(
                "Sentinel Turing timed out after bounded wait on current head {HEAD}, and Packaged Sentinel Euler: PASS on current head {HEAD}"
            ),
            format!(
                "Sentinel Turing has not returned on current head {HEAD}, and Packaged Sentinel Euler: PASS on current head {HEAD}"
            ),
            format!(
                "Sentinel Turing is running on current head {HEAD}, and Packaged Sentinel Euler: PASS on current head {HEAD}"
            ),
            format!(
                "Sentinel Turing timed out after bounded wait on current head {HEAD}, and Sentinel Euler is still running on current head {HEAD}, and Packaged Sentinel Turing: PASS on current head {HEAD}"
            ),
        ] {
            let current = format!("{history}, but {lifecycle}");
            let output = handoff(&current, false)?;
            assert!(
                !output.status.success(),
                "reviewer replacement or duplication must remain blocked"
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("changed or duplicated"),
                "unexpected reviewer-continuity result: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

#[test]
fn validator_accepts_alternate_nonterminal_forms_only_for_same_reviewer_terminal_result(
) -> TestResult {
    let prior = "Previous Packaged Sentinel Turing: UNOBSERVABLE after terminal tool failure on a prior run";
    for history in ["", prior] {
        for observation in ["has not returned", "is running"] {
            for terminal in ["PASS", "BLOCK"] {
                let lifecycle = format!(
                    "{history}, but Sentinel Turing {observation} on current head {HEAD}, and Packaged Sentinel Turing: {terminal} on current head {HEAD}"
                );
                let output = handoff(&lifecycle, terminal == "BLOCK")?;
                assert!(
                    output.status.success(),
                    "same-reviewer {observation} -> {terminal}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
    Ok(())
}

#[test]
fn validator_resets_continuity_after_terminal_verdicts() -> TestResult {
    let history = "Previous Packaged Sentinel Ada: UNOBSERVABLE on a prior run";
    for prior in ["", history] {
        for terminal in ["BLOCK", "UNOBSERVABLE"] {
            let lifecycle = format!(
                "{prior}, then Sentinel Turing timed out after bounded wait on current head {HEAD}, and Packaged Sentinel Turing: {terminal} on current head {HEAD}, and Packaged Sentinel Euler: PASS on current head {HEAD}"
            );
            let output = handoff(&lifecycle, false)?;
            assert!(
                output.status.success(),
                "terminal {terminal} must end Turing continuity before Euler PASS: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

#[test]
fn validator_scopes_historical_markers_to_reviewer_or_run_evidence() -> TestResult {
    let history = "Previous Packaged Sentinel Ada: UNOBSERVABLE on a prior run";
    for prior in ["", history] {
        for incidental in ["initial", "earlier", "old"] {
            let live = format!(
                "{prior}, then Sentinel Turing timed out after bounded wait during the {incidental} observation on current head {HEAD}, and Packaged Sentinel Euler: PASS on current head {HEAD}"
            );
            let output = handoff(&live, false)?;
            assert!(
                !output.status.success(),
                "incidental {incidental} wait text must not hide live Turing: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("changed or duplicated")
            );
        }

        let historical = format!(
            "{prior}, then Initial Packaged Sentinel Turing: BLOCK on an earlier run, and Packaged Sentinel Euler: PASS on current head {HEAD}"
        );
        let output = handoff(&historical, false)?;
        assert!(
            output.status.success(),
            "reviewer-qualified history must be excluded: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_complete_reviewer_identifier_tokens() -> TestResult {
    let history = "Previous Packaged Sentinel Ada: UNOBSERVABLE on a prior run";
    for prior in ["", history] {
        let replacement = format!(
            "{prior}, then Sentinel Review-1 timed out after bounded wait on current head {HEAD}, and Packaged Sentinel Review-2: PASS on current head {HEAD}"
        );
        let output = handoff(&replacement, false)?;
        assert!(
            !output.status.success(),
            "distinct suffixed reviewer identifiers must remain distinct: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("changed or duplicated")
        );

        let same = format!(
            "{prior}, then Sentinel Review_1 timed out after bounded wait on current head {HEAD}, and Packaged Sentinel Review_1: PASS on current head {HEAD}"
        );
        let output = handoff(&same, false)?;
        assert!(
            output.status.success(),
            "identical complete reviewer identifiers must stay continuous: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn handoff(status: &str, fallback: bool) -> TestResult<std::process::Output> {
    let approval = fallback
        .then_some(" Maintainer explicitly approved fallback for this Sentinel run.")
        .unwrap_or_default();
    let handoff = format!(
        "PR ready for parent handoff. {status}.{approval} Pushed: yes.\nBranch clean. Remote/PR head match: yes {HEAD}.\n"
    );
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(
        &pr_state_path,
        format!(
            "{{\"number\":489,\"state\":\"OPEN\",\"isDraft\":false,\"mergeStateStatus\":\"CLEAN\",\"reviewDecision\":\"APPROVED\",\"headRefName\":\"codexy/489\",\"headRefOid\":\"{HEAD}\",\"localHeadOid\":\"{HEAD}\",\"remoteHeadOid\":\"{HEAD}\",\"worktreeStatus\":\"## codexy/489...origin/codexy/489\",\"latestReviews\":[],\"reviewThreads\":{{\"pageInfo\":{{\"hasNextPage\":false}},\"nodes\":[]}}}}"
        ),
    )?;
    validate(&handoff_path, &pr_state_path)
}

fn validate(handoff: &Path, pr_state: &Path) -> TestResult<std::process::Output> {
    crate::support::validator_completion_handoff_files(handoff, pr_state)
}
