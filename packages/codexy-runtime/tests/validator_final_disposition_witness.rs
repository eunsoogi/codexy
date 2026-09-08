use std::process::Command;

use crate::support::TestResult;
#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/final_disposition.rs"]
mod final_support;

#[test]
fn current_919_witness_consumes_sanitized_same_head_disposition() -> TestResult {
    let (recovered, current) = final_support::recover_919()?;
    let control = final_support::final_control_919(&recovered)?;
    let incomplete_predecessor = final_support::produce_with_recovered_predecessor(
        &control,
        &current,
        &recovered,
        final_support::native_919_pull_request,
    )?;
    assert!(!incomplete_predecessor.status.success());
    let incomplete_diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&incomplete_predecessor.stdout),
        String::from_utf8_lossy(&incomplete_predecessor.stderr)
    );
    assert!(incomplete_diagnostic.contains("only valid after the third terminal verdict"));
    let produced = final_support::produce_with_states(
        &control,
        &current,
        &recovered,
        final_support::native_919_pull_request,
    )?;
    let state = final_support::build_pr_state_with_states(
        &control,
        &current,
        &recovered,
        final_support::native_919_pull_request,
    )?;

    assert_eq!(produced["issue_number"], final_support::native_919_issue);
    assert_eq!(produced["terminal_review_count"], 3);
    assert_eq!(produced["terminal_review_history"].as_array().map(Vec::len), Some(3));
    assert_eq!(produced["native_history_provenance"]["event_ids"],
        serde_json::json!([final_support::native_919_full_event, final_support::native_919_delta_event]));
    assert_eq!(produced["terminal_review_history"][0]["id"], final_support::native_919_full_event);
    assert_eq!(produced["terminal_review_history"][1]["id"], final_support::native_919_delta_event);
    assert_eq!(produced["terminal_review_history"][2]["id"], final_support::native_919_required_event);
    assert_eq!(produced["terminal_review_history"][0]["reviewer"]["model"], "gpt-5.6-sol");
    assert_eq!(produced["terminal_review_history"][0]["policy_reviewer"]["model"], "gpt-6-astra");
    assert!(!produced["terminal_review_history"][0]["unresolved_findings"].as_array().unwrap().is_empty());
    assert!(!produced["terminal_review_history"][1]["unresolved_findings"].as_array().unwrap().is_empty());
    assert_eq!(produced["terminal_review_history"][2]["unresolved_findings"][0]["id"], final_support::native_919_remaining_finding);
    assert_eq!(state["nativeHistoryRecovery"], recovered["nativeHistoryRecovery"]);
    assert_eq!(state["reviewControl"]["terminal_review_history"], produced["terminal_review_history"]);
    assert!(state["reviewControl"].get("native_history_recovery").is_none());
    let handoff = final_support::validate_handoff_state(
        &state,
        final_support::native_919_pull_request,
    )?;
    assert!(handoff.status.success(), "#919 native-history handoff failed: {}", String::from_utf8_lossy(&handoff.stderr));
    Ok(())
}

#[test]
fn direct_handoff_rejects_unobservable_final_disposition() -> TestResult {
    let mut control = final_support::same_head_control(993);
    control["terminal_result"] = serde_json::json!("UNOBSERVABLE");
    control["terminal_review_history"][2]["terminal_result"] = serde_json::json!("UNOBSERVABLE");
    let previous = final_support::third_block_predecessor(&control);
    let producer_error = final_support::produce(&control, &previous)
        .err()
        .ok_or("UNOBSERVABLE producer unexpectedly succeeded")?;
    assert!(producer_error
        .to_string()
        .contains("review control final disposition requires the authentic third BLOCK"));
    let handoff = final_support::validate_handoff_with_state(
        &control,
        993,
        direct_state::SYNTHETIC_CURRENT_HEAD,
        |_| Ok(()),
    )?;
    assert!(!handoff.status.success());
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&handoff.stdout),
        String::from_utf8_lossy(&handoff.stderr)
    );
    assert!(
        diagnostic.contains("review control final disposition requires the authentic third BLOCK"),
        "UNOBSERVABLE handoff was rejected for an unexpected reason: {diagnostic}"
    );
    Ok(())
}

#[test]
fn historical_baseline_and_current_replay_use_the_same_valid_input() -> TestResult {
    let (recovered, current) = final_support::recover_919()?;
    let control = final_support::final_control_919(&recovered)?;
    let root = tempfile::tempdir()?;
    let repository = codexy_runtime::paths::repository_root();
    let source = root.path().join("baseline-source");
    let target = root.path().join("baseline-target");
    let baseline = "cae9d65abaf4188431f0451a9d2161091f94012b";
    let commit_label = |revision: &str| -> TestResult<String> {
        let output = Command::new("git")
            .args(["-C", repository.to_str().ok_or("repository path")?, "show", "-s", "--format=%H %cI"])
            .arg(revision)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "commit metadata lookup failed for {revision}: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_owned())
    };
    let baseline_label = commit_label(baseline)?;
    let current_label = commit_label("HEAD")?;
    let add = Command::new("git")
        .args(["-C", repository.to_str().ok_or("repository path")?, "worktree", "add", "--detach"])
        .arg(&source)
        .arg(baseline)
        .output()?;
    assert!(
        add.status.success(),
        "historical baseline worktree failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    let build = Command::new("cargo")
        .args(["build", "--manifest-path"])
        .arg(source.join("packages/codexy-runtime/Cargo.toml"))
        .args(["--bin", "codexy-review-control", "--locked", "--target-dir"])
        .arg(&target)
        .output()?;
    assert!(
        build.status.success(),
        "historical baseline build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let binary = target.join("debug").join(if cfg!(windows) {
        "codexy-review-control.exe"
    } else {
        "codexy-review-control"
    });
    let cleanup = Command::new("git")
        .args(["-C", repository.to_str().ok_or("repository path")?, "worktree", "remove", "--force"])
        .arg(&source)
        .status()?;
    assert!(cleanup.success(), "historical baseline worktree cleanup failed");
    let legacy = final_support::produce_with_binary(
        &control,
        &current,
        &recovered,
        final_support::native_919_pull_request,
        &binary,
    )?;
    assert!(!legacy.status.success());
    let legacy_diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&legacy.stdout),
        String::from_utf8_lossy(&legacy.stderr)
    );
    assert!(
        legacy_diagnostic.contains("review control transition must append exactly one terminal event"),
        "historical baseline rejected the same valid input for an unexpected reason: {legacy_diagnostic}"
    );
    let current_result = final_support::produce_with_states(
        &control,
        &current,
        &recovered,
        final_support::native_919_pull_request,
    )?;
    assert_eq!(current_result["terminal_review_count"], 3);
    eprintln!(
        "historical replay: baseline {baseline_label} rejected; current {current_label} accepted the same valid input"
    );
    Ok(())
}
