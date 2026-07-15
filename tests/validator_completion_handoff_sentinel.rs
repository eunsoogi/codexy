use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

const HEAD: &str = "32b03a210b3defb2d29dd352283ea2488e60d893";

#[test]
fn validator_rejects_open_pr_completion_claims_even_with_sentinel_evidence() -> TestResult {
    for handoff in [
        format!("Completed. Sentinel: PASS on current head {HEAD}.\n"),
        format!("Finished. Sentinel: BLOCK on current head {HEAD}.\n"),
        format!("Finalized. Sentinel: UNOBSERVABLE after bounded wait on current head {HEAD}.\n"),
    ] {
        reject_open_pr_completion_handoff(&handoff)?;
    }
    Ok(())
}

#[test]
fn validator_keeps_deferrals_and_readiness_distinct_from_completion() -> TestResult {
    for handoff in [
        format!(
            "Maintainer requested no-merge; Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. Work is complete after PR #128.\n"
        ),
        format!(
            "PR ready for parent handoff. Packaged Codexy Sentinel Turing: PASS on current head {HEAD}.\n"
        ),
    ] {
        accept_open_pr_handoff(&handoff)?;
    }
    Ok(())
}

fn reject_open_pr_completion_handoff(handoff: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        !output.status.success(),
        "validator should reject open-PR completion handoff\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("opening a PR is not completion"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn accept_open_pr_handoff(handoff: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        output.status.success(),
        "validator should accept non-completion or explicitly deferred handoff\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_completion_handoff(handoff_path: &Path, pr_state_path: &Path) -> OutputResult {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-completion-handoff",
            "--handoff-file",
            handoff_path.to_str().ok_or("handoff path")?,
            "--pr-state-file",
            pr_state_path.to_str().ok_or("pr state path")?,
        ])
        .output()?)
}

fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(
        &pr_state_path,
        format!(
            r###"{{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/221-sentinel-bounded-wait-status","headRefOid":"{HEAD}","localHeadOid":"{HEAD}","remoteHeadOid":"{HEAD}","worktreeStatus":"## codexy/221-sentinel-bounded-wait-status...origin/codexy/221-sentinel-bounded-wait-status","latestReviews":[{{"body":"Didn't find any major issues.\n\nReviewed commit: `{HEAD}`","author":{{"login":"automated-review"}},"submittedAt":"2026-07-03T00:00:00Z","commit":{{"oid":"{HEAD}"}}}}],"reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}}}"###
        ),
    )?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}
