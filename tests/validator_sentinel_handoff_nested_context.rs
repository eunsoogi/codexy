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
