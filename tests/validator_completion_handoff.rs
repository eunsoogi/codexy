use std::{path::Path, process::Command};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;
#[test]
fn validator_cli_rejects_completion_claim_with_clean_open_pr() -> TestResult {
    reject_open_pr_completion_handoff(
        "Implemented the fix, verified it, and opened PR #128. Work is complete.\n",
        "validator should reject completion claims while a matching clean PR remains open",
    )
}
#[test]
fn validator_cli_rejects_completion_claim_with_open_pr() -> TestResult {
    for (merge_state, is_draft) in [
        ("UNKNOWN", false),
        ("BLOCKED", false),
        ("UNSTABLE", false),
        ("UNKNOWN", true),
    ] {
        let output = validate_handoff_with_pr_state(
            "Work is complete after PR #128.\n",
            &format!(
                r#"{{"number":128,"state":"OPEN","isDraft":{is_draft},"mergeStateStatus":"{merge_state}","reviewDecision":"APPROVED"}}"#
            ),
        )?;
        assert!(
            !output.status.success(),
            "validator should fail closed for open PR state {merge_state} draft={is_draft}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("opening a PR is not completion"));
    }
    Ok(())
}
#[test]
fn validator_cli_allows_explicit_stop_condition_with_clean_open_pr() -> TestResult {
    for handoff in [
        "Draft PR #128 is open per the stop condition. Parent orchestrator will handle review and merge gates; this lane is not complete.\n",
        "Maintainer requested push only; Packaged Codexy Sentinel Turing: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Work is complete after PR #128.\n",
        "Maintainer requested draft only; Packaged Codexy Sentinel Turing: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Work is complete after PR #128.\n",
        "Maintainer requested no-merge; Packaged Codexy Sentinel Turing: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Work is complete after PR #128.\n",
        "Maintainer requested leave-open; Packaged Codexy Sentinel Turing: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Work is complete after PR #128.\n",
        "Maintainer asked me to leave open no matter what. Packaged Codexy Sentinel Turing: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Work is complete after PR #128.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should allow explicit defer/stop handoffs for open PRs",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_rejects_completion_claim_that_only_says_waiting_for_merge() -> TestResult {
    reject_open_pr_completion_handoff(
        "Work is complete. Waiting for merge after PR #128.\n",
        "validator should reject completion claims that merely say they are waiting for merge",
    )
}
#[test]
fn validator_cli_rejects_completion_claim_that_negates_explicit_stop() -> TestResult {
    reject_open_pr_completion_handoff(
        "No explicit stop condition was requested. Work is complete. Waiting for merge after PR #128.\n",
        "validator should reject completion claims that negate explicit stop instructions",
    )
}
#[test]
fn validator_cli_rejects_empty_stop_condition_label() -> TestResult {
    reject_open_pr_completion_handoff(
        "Stop condition: none. Work is complete after PR #128.\n",
        "validator should reject stop-condition labels without real deferral text",
    )
}
#[test]
fn validator_cli_rejects_missing_pr_state_fields() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Work is complete after PR #128.\n",
        r#"{"number":128,"state":"OPEN","isDraft":false,"reviewDecision":"APPROVED"}"#,
    )?;
    assert!(
        !output.status.success(),
        "validator should fail closed on incomplete PR state\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR state"));
    Ok(())
}
#[test]
fn validator_cli_rejects_empty_no_merge_instruction_labels() -> TestResult {
    for handoff in [
        "No-merge instruction: none. Work is complete after PR #128.\n",
        "No-merge instruction: false. Work is complete after PR #128.\n",
        "No-merge instruction: not requested. Work is complete after PR #128.\n",
        "No-merge instruction not requested. Work is complete after PR #128.\n",
        "Draft-only instruction not requested. Work is complete after PR #128.\n",
        "No-merge instruction: no. Work is complete after PR #128.\n",
        "No-merge instruction: N/A. Work is complete after PR #128.\n",
        "Maintainer requested wait: not required. Work is complete after PR #128.\n",
        "No-merge instruction was requested. Work is complete after PR #128.\n",
        "No-merge instruction was requested by parent orchestrator. Work is complete after PR #128.\n",
        "No-merge instruction: maintainer did not request no merge. Work is complete after PR #128.\n",
        "No-merge instruction: maintainer requested a Codex review only. Work is complete after PR #128.\n",
        "No-merge instruction from maintainer was not requested. Work is complete after PR #128.\n",
        "No-merge instruction. Work is complete after PR #128.\n",
        "Draft-only instruction\nWork is complete after PR #128.\n",
        "No-merge instruction\nNone.\nWork is complete after PR #128.\n",
        "Maintainer requested no merge? No. Work is complete after PR #128.\n",
        "Maintainer requested no merge? false. Work is complete after PR #128.\n",
        "No-merge instruction:\nWork is complete after PR #128.\n",
        "Draft-only instruction: not applicable. Work is complete after PR #128.\n",
        "Draft-only instruction: maintainer requested a Codex review only. Work is complete after PR #128.\n",
        "Draft-only instruction was not requested. Work is complete after PR #128.\n",
        "No explicit stop, wait, draft-only, no-merge instruction was requested. Work is complete after PR #128.\n",
        "No explicit stop, wait, draft-only, or no-merge instruction was requested. Work is complete after PR #128.\n",
        "No explicit stop, wait, draft-only or no-merge instruction was requested. Work is complete after PR #128.\n",
        "No explicit stop or no-merge instruction was requested. Work is complete after PR #128.\n",
        "No explicit stop, no-merge instruction was requested. Work is complete after PR #128.\n",
        "No explicit stop, wait, no-merge instruction was requested. Work is complete after PR #128.\n",
        "No-merge instruction is not requested. Work is complete after PR #128.\n",
        "No-merge instruction: . Work is complete after PR #128.\n",
    ] {
        reject_open_pr_completion_handoff(
            handoff,
            "validator should reject no-merge labels without real deferral text",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_rejects_false_or_unrelated_deferrals() -> TestResult {
    for handoff in [
        "No maintainer explicitly requested stop or wait. Work is complete. Waiting for merge after PR #128.\n",
        "The parent orchestrator asked me to wait for merge gates. Work is complete after PR #128.\n",
        "- [ ] **maintainer requested wait**\nWork is complete after PR #128.\n",
        "Maintainer explicitly requested a Codex review. Work is complete after PR #128.\n",
        "Maintainer requested no merge = false. Work is complete after PR #128.\n",
        "No maintainer explicitly requested no merge. Work is complete after PR #128.\n",
        "No maintainer explicitly requested leave open. Work is complete after PR #128.\n",
        "No user or maintainer requested no merge. Work is complete after PR #128.\n",
        "No draft-only instruction was requested. Work is complete after PR #128.\n",
        "No explicit draft-only instruction was requested. Work is complete after PR #128.\n",
        "The maintainer did not ask me to leave open. Work is complete after PR #128.\n",
    ] {
        reject_open_pr_completion_handoff(
            handoff,
            "validator should reject false or unrelated deferrals",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_rejects_natural_completion_claims_after_pr() -> TestResult {
    for handoff in [
        "The lane is complete after PR #128.\n",
        "The implementation is complete after PR #128.\n",
        "This completes the task after PR #128.\n",
        "Complete",
        "Complete!\n",
        "Complete after opening PR #128.\n",
        "Complete after PR #128.\n",
        "Opened PR #128. Finished.\n",
        "Opened PR #128. Finished. This lane is not complete until merge.\n",
        "Verification completed; this lane is not complete until merge. Work completed.\n",
        "Verification finished; this lane is not complete until merge. Finished.\n",
        "Opened PR #128. Finalized.\n",
        "Done",
        "Done — PR #128 is open.\n",
        "Done after opening PR #128.\n",
        "Done after PR #128.\n",
        "Implemented the fix and opened PR #128. Complete.\n",
        "Work is complete. Parent orchestrator will handle review and merge gates.\n",
        "Work is complete; PR #128 is open per the stop condition.\n",
        "No blockers. Work is complete.\n",
        "검토 완료입니다. Work is complete.\n",
        "검토🙂완료입니다. Work is complete.\n",
    ] {
        reject_open_pr_completion_handoff(
            handoff,
            "validator should reject natural completion claims after opening a PR",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_accepts_negated_completion_claim_after_pr() -> TestResult {
    for handoff in [
        "This lane is not complete after PR #128.\n",
        "This lane is incomplete after PR #128.\n",
        "We aren't complete.\n",
        "This lane is not yet complete until merge.\n",
        "Verification completed successfully; this lane is not complete until merge.\n",
        "Verification completed. This lane is not complete until merge.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should allow accurate non-completion text",
        )?;
    }
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
fn reject_open_pr_completion_handoff(handoff: &str, failure_message: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        !output.status.success(),
        "{failure_message}\nstdout:\n{}\nstderr:\n{}",
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
fn accept_open_pr_handoff(handoff: &str, failure_message: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        output.status.success(),
        "{failure_message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}
fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    validate_handoff_with_pr_state(
        handoff,
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#,
    )
}
