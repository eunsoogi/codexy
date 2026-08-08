type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const HEAD: &str = "32b03a210b3defb2d29dd352283ea2488e60d893";

#[test]
fn validator_keeps_nested_inactive_terminal_results_out_of_live_continuity() -> TestResult {
    for inactive in [
        "- > Packaged Sentinel Turing: {status} on current head {HEAD}",
        "1. > Packaged Sentinel Turing: {status} on current head {HEAD}",
        "- - > Packaged Sentinel Turing: {status} on current head {HEAD}",
        "- ```text\n- Packaged Sentinel Turing: {status} on current head {HEAD}\n- ```",
        "1. ```text\n1. Packaged Sentinel Turing: {status} on current head {HEAD}\n1. ```",
        "- - ```text\n- - Packaged Sentinel Turing: {status} on current head {HEAD}\n- - ```",
    ] {
        for status in ["PASS", "BLOCK", "UNOBSERVABLE"] {
            let inactive = inactive.replace("{status}", status).replace("{HEAD}", HEAD);
            let output = handoff(
                &format!("Sentinel Turing is still running on current head {HEAD}.\n{inactive}"),
            )?;
            assert!(
                !output.status.success(),
                "nested inactive {status} result must not end live continuity"
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("still running"),
                "unexpected nested inactive result: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

#[test]
fn validator_orders_active_generic_terminal_vetoes_after_named_evidence() -> TestResult {
    for veto in [
        "Sentinel: BLOCK",
        "Sentinel: UNOBSERVABLE",
        "Reviewer gate: BLOCK",
        "Reviewer gate returned BLOCK",
        "Reviewer gate BLOCK",
        "Reviewer gate verdict: BLOCK",
        "Reviewer gate result: BLOCK",
        "Reviewer-gate returned BLOCK",
        "Reviewer-gate BLOCK",
        "Reviewer-gate verdict: BLOCK",
        "Reviewer-gate result: BLOCK",
        "- Reviewer gate: BLOCK",
    ] {
        assert_rejected(&named_terminal("PASS", veto))?;
    }
    assert_rejected(&named_terminal("BLOCK", "Reviewer gate: PASS"))?;
    for ignored in [
        "Earlier reviewer gate returned BLOCK. Packaged Sentinel Turing: PASS on current head {HEAD}",
        "- [ ] Reviewer gate returned BLOCK\nPackaged Sentinel Turing: PASS on current head {HEAD}",
        "Packaged Sentinel Turing: PASS on current head {HEAD}\n- > Reviewer gate: BLOCK",
        "Packaged Sentinel Turing: PASS on current head {HEAD}\n- ```text\n- Reviewer gate: BLOCK\n- ```",
        "Packaged Sentinel Turing: PASS on current head {HEAD}. Reviewer-gate result: documentation note only",
    ] {
        assert_accepted(ignored)?;
    }
    Ok(())
}

fn named_terminal(named: &str, later: &str) -> String {
    format!(
        "Packaged Sentinel Turing: {named} on current head {HEAD}. {later} on current head {HEAD}"
    )
}

fn assert_rejected(status: &str) -> TestResult {
    let output = handoff(status)?;
    assert!(
        !output.status.success(),
        "active generic terminal status must veto named evidence: {status}"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Sentinel"));
    Ok(())
}

fn assert_accepted(status: &str) -> TestResult {
    let output = handoff(&status.replace("{HEAD}", HEAD))?;
    assert!(
        output.status.success(),
        "inactive or explanatory generic status must not veto named evidence: {status}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn handoff(status: &str) -> TestResult<std::process::Output> {
    let handoff = format!(
        "PR ready for parent handoff. {status}. Pushed: yes.\nBranch clean. Remote/PR head match: yes {HEAD}.\n"
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
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}
