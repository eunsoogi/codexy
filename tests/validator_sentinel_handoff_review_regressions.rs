use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const HEAD: &str = "32b03a210b3defb2d29dd352283ea2488e60d893";

#[test]
fn validator_rejects_later_generic_block_after_named_sentinel_pass() -> TestResult {
    for verdict in [
        "Reviewer gate: BLOCK",
        "Reviewer gate returned BLOCK",
        "Reviewer gate BLOCK",
        "Reviewer gate verdict: BLOCK",
        "Reviewer-gate returned BLOCK",
        "Reviewer-gate BLOCK",
        "Reviewer-gate verdict: BLOCK",
    ] {
        let handoff = format!(
            "Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. {verdict} on current head {HEAD}. PR ready for parent handoff. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        );
        let output = validate_file(&handoff)?;
        assert!(
            !output.status.success(),
            "validator accepted a later generic reviewer-gate BLOCK after named Sentinel PASS\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("BLOCK"),
            "unexpected stderr: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_ignores_non_current_generic_block_text() -> TestResult {
    for context in [
        "Earlier reviewer gate returned BLOCK. Packaged Codexy Sentinel Turing: PASS",
        "- [ ] Reviewer gate returned BLOCK\nPackaged Codexy Sentinel Turing: PASS",
        "Reviewer gate returned BLOCKER. Packaged Codexy Sentinel Turing: PASS",
    ] {
        assert_accepts(&format!(
            "{context} on current head {HEAD}. PR ready for parent handoff. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        ))?;
    }
    for context in [
        "Example: Reviewer gate returned BLOCK",
        "Stale reviewer gate returned BLOCK",
    ] {
        assert_accepts(&format!(
            "Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. {context}. PR ready for parent handoff. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        ))?;
    }
    Ok(())
}

#[test]
fn validator_ignores_generic_block_text_in_explicit_examples() -> TestResult {
    for context in [
        "Example 1: Reviewer gate returned BLOCK",
        "Historical example: Reviewer gate returned BLOCK",
        "### Example\nReviewer gate returned BLOCK",
    ] {
        assert_accepts(&format!(
            "Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. {context}. PR ready for parent handoff. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        ))?;
    }
    Ok(())
}

#[test]
fn validator_rejects_current_generic_block_after_example_text() -> TestResult {
    let handoff = format!(
        "Packaged Codexy Sentinel Turing: PASS on current head {HEAD}. ### Example\nReviewer gate returned BLOCK. Reviewer gate returned BLOCK on current head {HEAD}. PR ready for parent handoff. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
    );
    assert_rejects(&handoff)
}

#[test]
fn validator_rejects_generic_reviewer_gate_as_packaged_sentinel_proof() -> TestResult {
    let output = validate_file(&format!(
        "PR ready for parent handoff. Reviewer gate: PASS on current head {HEAD}. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
    ))?;
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("Generic reviewer-gate evidence"),
        "unexpected stderr: {}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn validator_rejects_missing_sentinel_pass_evidence() -> TestResult {
    for evidence in [
        "PASS: missing evidence",
        "PASS: evidence missing",
        "PASS was missing",
        "PASS status was missing",
        "PASS evidence was missing",
        "PASS proof was absent",
    ] {
        assert_rejects(&format!(
            "PR ready for parent handoff. Packaged Codexy Sentinel Turing: {evidence} on current head {HEAD}. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
        ))?;
    }
    Ok(())
}

#[test]
fn validator_accepts_unrelated_missing_after_named_sentinel_pass() -> TestResult {
    assert_accepts(&format!(
        "PR ready for parent handoff. Packaged Codexy Sentinel Turing: PASS after fixing missing tests on current head {HEAD}. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
    ))
}

#[test]
fn validator_accepts_explanatory_helper_verb_missing_after_named_sentinel_pass() -> TestResult {
    assert_accepts(&format!(
        "PR ready for parent handoff. Packaged Codexy Sentinel Turing: PASS after fixing a fixture that was missing on current head {HEAD}. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
    ))
}

#[test]
fn validator_rejects_role_only_sentinel_reviewer_gate_label() -> TestResult {
    let output = validate_file(&format!(
        "PR ready for parent handoff. Packaged Codexy Sentinel reviewer gate: PASS on current head {HEAD}. Branch clean. Pushed at {HEAD}. Remote/PR head match: yes {HEAD}.\n"
    ))?;
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("must name the packaged Sentinel reviewer"),
        "unexpected stderr: {}",
        stderr(&output)
    );
    Ok(())
}

fn assert_rejects(handoff: &str) -> TestResult {
    let output = validate_file(handoff)?;
    assert!(
        !output.status.success(),
        "validator accepted missing Sentinel evidence\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    assert!(stderr(&output).contains("Sentinel"));
    Ok(())
}

fn assert_accepts(handoff: &str) -> TestResult {
    let output = validate_file(handoff)?;
    assert!(
        output.status.success(),
        "validator rejected valid Sentinel evidence\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
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
        r###"{{"number":221,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/221-sentinel-bounded-wait-status","headRefOid":"{HEAD}","localHeadOid":"{HEAD}","remoteHeadOid":"{HEAD}","worktreeStatus":"## codexy/221-sentinel-bounded-wait-status...origin/codexy/221-sentinel-bounded-wait-status","latestReviews":[{{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b`","author":{{"login":"chatgpt-codex-connector"}},"submittedAt":"2026-07-03T00:00:00Z"}}],"reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}}}"###
    )
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
