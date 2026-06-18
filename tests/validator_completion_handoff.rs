use std::process::Command;

#[test]
fn validator_cli_rejects_completion_claim_with_clean_open_pr()
-> Result<(), Box<dyn std::error::Error>> {
    reject_open_pr_completion_handoff(
        "Implemented the fix, verified it, and opened PR #128. Work is complete.\n",
        "validator should reject completion claims while a matching clean PR remains open",
    )
}

#[test]
fn validator_cli_allows_explicit_stop_condition_with_clean_open_pr()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(
        &handoff_path,
        "Draft PR #128 is open per the stop condition. Parent orchestrator will handle review and merge gates; this lane is not complete.\n",
    )?;
    std::fs::write(
        &pr_state_path,
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#,
    )?;

    let output = validate_completion_handoff(&handoff_path, &pr_state_path)?;
    assert!(
        output.status.success(),
        "validator should allow explicit defer/stop handoffs for open PRs\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_completion_claim_that_only_says_waiting_for_merge()
-> Result<(), Box<dyn std::error::Error>> {
    reject_open_pr_completion_handoff(
        "Work is complete. Waiting for merge after PR #128.\n",
        "validator should reject completion claims that merely say they are waiting for merge",
    )
}

#[test]
fn validator_cli_rejects_completion_claim_that_negates_explicit_stop()
-> Result<(), Box<dyn std::error::Error>> {
    reject_open_pr_completion_handoff(
        "No explicit stop condition was requested. Work is complete. Waiting for merge after PR #128.\n",
        "validator should reject completion claims that negate explicit stop instructions",
    )
}

#[test]
fn validator_cli_rejects_completion_claim_with_negated_maintainer_request()
-> Result<(), Box<dyn std::error::Error>> {
    reject_open_pr_completion_handoff(
        "No maintainer explicitly requested stop or wait. Work is complete. Waiting for merge after PR #128.\n",
        "validator should reject completion claims that negate a maintainer stop/wait request",
    )
}

#[test]
fn validator_cli_rejects_unrelated_maintainer_request_as_deferral()
-> Result<(), Box<dyn std::error::Error>> {
    reject_open_pr_completion_handoff(
        "Maintainer explicitly requested a Codex review. Work is complete after PR #128.\n",
        "validator should reject unrelated maintainer requests as merge deferrals",
    )
}

#[test]
fn validator_cli_rejects_negated_no_merge_deferral() -> Result<(), Box<dyn std::error::Error>> {
    reject_open_pr_completion_handoff(
        "No maintainer explicitly requested no merge. Work is complete after PR #128.\n",
        "validator should reject negated no-merge deferrals",
    )
}

#[test]
fn validator_cli_rejects_negated_leave_open_deferral() -> Result<(), Box<dyn std::error::Error>> {
    reject_open_pr_completion_handoff(
        "No maintainer explicitly requested leave open. Work is complete after PR #128.\n",
        "validator should reject negated leave-open deferrals",
    )
}

#[test]
fn validator_cli_rejects_negated_draft_only_instruction() -> Result<(), Box<dyn std::error::Error>>
{
    reject_open_pr_completion_handoff(
        "No draft-only instruction was requested. Work is complete after PR #128.\n",
        "validator should reject negated draft-only instructions",
    )
}

#[test]
fn validator_cli_rejects_maintainer_did_not_ask_leave_open()
-> Result<(), Box<dyn std::error::Error>> {
    reject_open_pr_completion_handoff(
        "The maintainer did not ask me to leave open. Work is complete after PR #128.\n",
        "validator should reject maintainer leave-open denials",
    )
}

#[test]
fn validator_cli_rejects_natural_completion_claims_after_pr()
-> Result<(), Box<dyn std::error::Error>> {
    for handoff in [
        "The lane is complete after PR #128.\n",
        "The implementation is complete after PR #128.\n",
        "Complete after opening PR #128.\n",
        "Complete after PR #128.\n",
        "Done after opening PR #128.\n",
        "Done after PR #128.\n",
        "Work is complete. Parent orchestrator will handle review and merge gates.\n",
    ] {
        reject_open_pr_completion_handoff(
            handoff,
            "validator should reject natural completion claims after opening a PR",
        )?;
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_negated_completion_claim_after_pr()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, "This lane is not complete after PR #128.\n")?;
    std::fs::write(
        &pr_state_path,
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#,
    )?;

    let output = validate_completion_handoff(&handoff_path, &pr_state_path)?;
    assert!(
        output.status.success(),
        "validator should allow negated completion text\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_completion_handoff(
    handoff_path: &std::path::Path,
    pr_state_path: &std::path::Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
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

fn reject_open_pr_completion_handoff(
    handoff: &str,
    failure_message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(
        &pr_state_path,
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#,
    )?;

    let output = validate_completion_handoff(&handoff_path, &pr_state_path)?;
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
