use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const HEAD: &str = "32b03a210b3defb2d29dd352283ea2488e60d893";

#[test]
fn validator_rejects_generic_sentinel_pass_without_reviewer_name() -> TestResult {
    for handoff in [
        format!(
            "PR ready for parent handoff. Sentinel: PASS on current head {HEAD}. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        ),
        format!(
            "PR ready for parent handoff. Sentinel returned PASS on current head {HEAD}. Alice approved docs separately. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        ),
    ] {
        assert_rejects_sentinel_handoff(&handoff)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_child_handoff_readiness_with_blocked_sentinel() -> TestResult {
    for handoff in [
        format!(
            "Child handoff: branch clean, synced, and pushed at {HEAD}. Sentinel: BLOCK on current head {HEAD}.\n"
        ),
        format!(
            "Child handoff: branch clean, synced, and pushed at {HEAD}. Sentinel: UNOBSERVABLE after bounded wait on current head {HEAD}.\n"
        ),
        format!(
            "Child handoff: Clean: yes. Synced: yes. Pushed: yes at {HEAD}. Sentinel: BLOCK on current head {HEAD}.\n"
        ),
        format!(
            "Child handoff: branch clean and pushed at {HEAD}. Parent can merge. Sentinel: UNOBSERVABLE after bounded wait on current head {HEAD}.\n"
        ),
    ] {
        assert_rejects_sentinel_handoff(&handoff)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pr_readiness_with_blocked_sentinel() -> TestResult {
    for handoff in [
        format!("PR is ready. Sentinel: BLOCK on current head {HEAD}.\n"),
        format!("PR is ready. Sentinel: UNOBSERVABLE after bounded wait on current head {HEAD}.\n"),
    ] {
        assert_rejects_sentinel_handoff(&handoff)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_modal_future_sentinel_pass_after_block() -> TestResult {
    for handoff in [
        format!(
            "PR ready for parent handoff. Sentinel: BLOCK on current head. Packaged Codexy Sentinel Turing will return PASS on current head {HEAD} after rerun.\n"
        ),
        format!(
            "PR ready for parent handoff. Sentinel: BLOCK on current head. Packaged Codexy Sentinel Turing is expected to PASS on current head {HEAD} after rerun.\n"
        ),
    ] {
        assert_rejects_sentinel_handoff(&handoff)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_reviewer_gate_result_block_after_sentinel_pass() -> TestResult {
    for handoff in [
        format!(
            "PR ready for parent handoff. Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. Reviewer gate result: BLOCK on current head. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        ),
        format!(
            "PR ready for parent handoff. Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. Reviewer-gate result: BLOCK on current head. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        ),
    ] {
        assert_rejects_sentinel_handoff(&handoff)?;
    }
    Ok(())
}

#[test]
fn validator_accepts_explanatory_reviewer_gate_result_text_after_sentinel_pass() -> TestResult {
    let handoff = format!(
        "PR ready for parent handoff. Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. Reviewer-gate result: documentation note only. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
    );
    let output = validate_file(&handoff)?;
    assert!(
        output.status.success(),
        "validator should preserve Sentinel PASS when reviewer-gate result text is explanatory\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_reviewer_named_returned_pass() -> TestResult {
    let handoff = format!(
        "PR ready for parent handoff. Packaged Codexy Sentinel Turing returned PASS on current head {HEAD}. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
    );
    let output = validate_file(&handoff)?;
    assert!(
        output.status.success(),
        "validator should accept reviewer-named returned PASS evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_rejects_sentinel_handoff(handoff: &str) -> TestResult {
    let output = validate_file(handoff)?;
    assert!(
        !output.status.success(),
        "validator should reject invalid Sentinel handoff\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Sentinel"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_file(handoff: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, open_pr_state())?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-completion-handoff",
            "--handoff-file",
            path_str(&handoff_path)?,
            "--pr-state-file",
            path_str(&pr_state_path)?,
        ])
        .output()?)
}

fn path_str(path: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    path.to_str()
        .ok_or_else(|| "path is not valid UTF-8".into())
}

fn open_pr_state() -> String {
    format!(
        r###"{{"number":221,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/221-sentinel-bounded-wait-status","headRefOid":"{HEAD}","localHeadOid":"{HEAD}","remoteHeadOid":"{HEAD}","worktreeStatus":"## codexy/221-sentinel-bounded-wait-status...origin/codexy/221-sentinel-bounded-wait-status","latestReviews":[{{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b`","author":{{"login":"automated-review"}},"submittedAt":"2026-07-03T00:00:00Z"}}],"reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}}}"###
    )
}
