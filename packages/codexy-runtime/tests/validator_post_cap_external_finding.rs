use std::{fs, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;

#[path = "support/post_cap_review.rs"]
mod post_cap;
#[path = "support/post_cap_external_finding_fixture.rs"]
mod external_finding_fixture;

use external_finding_fixture::pr938_finding;

const FULL_HEAD: &str = direct_state::SYNTHETIC_FULL_HEAD;
const DELTA_HEAD: &str = direct_state::SYNTHETIC_DELTA_HEAD;
const CURRENT_HEAD: &str = direct_state::SYNTHETIC_CURRENT_HEAD;
const BASE: &str = direct_state::SYNTHETIC_BASE;
const EVIDENCE: &str = direct_state::SYNTHETIC_EXTERNAL_EVIDENCE;
const FINDING_ID: &str = "github-pr938-discussion-r3940672308";

#[test]
fn post_cap_re_review_accepts_authenticated_external_finding_repair_after_delta_pass() -> TestResult {
    let finding_id = "github-pr938-discussion-r3940672308";
    let mut control = direct_state::post_cap_control_with_findings(
        947,
        FULL_HEAD,
        DELTA_HEAD,
        CURRENT_HEAD,
        "authenticated_external_finding_repair",
        EVIDENCE,
        "PASS",
        json!([]),
        json!([finding_id]),
    );
    control["post_cap_re_review"]["qualifying_change"]["external_finding"] =
        pr938_finding(DELTA_HEAD);

    let state = post_cap::build_pr_state(&control, BASE, BASE)?;
    assert_eq!(state["reviewControl"]["terminal_review_count"], 3);
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][1]["terminal_result"],
        "PASS"
    );
    assert_eq!(
        state["reviewControl"]["post_cap_re_review"]["reason"],
        "authenticated_external_finding_repair"
    );
    assert_eq!(
        state["reviewControl"]["post_cap_re_review"]["qualifying_change"]["finding_ids"],
        json!([finding_id])
    );
    Ok(())
}

#[test]
fn post_cap_re_review_accepts_blocked_full_then_clean_delta() -> TestResult {
    let mut control = direct_state::post_cap_control_with_findings(
        947,
        FULL_HEAD,
        DELTA_HEAD,
        CURRENT_HEAD,
        "authenticated_external_finding_repair",
        EVIDENCE,
        "PASS",
        json!([]),
        json!([FINDING_ID]),
    );
    control["terminal_review_history"][0]["terminal_result"] = json!("BLOCK");
    control["terminal_review_history"][0]["unresolved_findings"] = json!([{
        "id": "historical-full-finding",
        "path": "README.md"
    }]);
    control["post_cap_re_review"]["qualifying_change"]["external_finding"] =
        pr938_finding(DELTA_HEAD);

    let state = post_cap::build_pr_state(&control, BASE, BASE)?;
    assert_eq!(state["reviewControl"]["terminal_review_history"][0]["terminal_result"], "BLOCK");
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][1]["terminal_result"],
        "PASS"
    );
    Ok(())
}

#[test]
#[ignore = "requires an authenticated live GitHub read"]
fn producer_accepts_actual_pr938_discussion_on_its_reviewed_commit() -> TestResult {
    let repository = codexy_runtime::paths::repository_root();
    let delta = git(repository, &["rev-parse", "0c82aedc4748cb40cacaecb08a946b2a8628f8ab"])?;
    let full = git(repository, &["rev-parse", "0c82aedc4748cb40cacaecb08a946b2a8628f8ab^"])?;
    let current = git(repository, &["rev-parse", "HEAD"])?;
    let mut control = direct_state::post_cap_control_with_findings(
        947,
        &full,
        &delta,
        &current,
        "authenticated_external_finding_repair",
        &current,
        "PASS",
        json!([]),
        json!(["github-pr938-discussion-r3940672308"]),
    );
    let change = control["post_cap_re_review"]["qualifying_change"]
        .as_object_mut()
        .ok_or("qualifying change")?;
    change.remove("external_finding");
    change.remove("finding_ids");
    let previous = direct_state::post_cap_prior(&control);
    let base = git(repository, &["rev-parse", "0c82aedc4748cb40cacaecb08a946b2a8628f8ab^"])?;
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_external_finding_locator": {
                "repository": "eunsoogi/codexy",
                "owningIssue": 937,
                "pullRequest": 938,
                "reviewThread": "PRRT_kwDOS6i-_86fjYep",
                "reviewComment": "PRRC_kwDOS6i-_87q4eM0"
            },
            "current_pr_state": direct_state::pr_snapshot(947, &base, &current, None),
            "previous_pr_state": direct_state::pr_snapshot(947, &base, &delta, Some(previous.clone()))
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .args(["--repository-root"])
        .arg(repository)
        .status()?;
    assert!(result.success(), "actual PR938 source must pass producer");
    let produced: Value = serde_json::from_slice(&fs::read(output)?)?;
    assert_eq!(produced["terminal_review_count"], 3);
    assert_eq!(
        produced["post_cap_re_review"]["qualifying_change"]["external_finding"]
            ["reviewThread"]["id"],
        "PRRT_kwDOS6i-_86fjYep"
    );
    let source = &produced["post_cap_re_review"]["qualifying_change"]["external_finding"];
    eprintln!("live GitHub source: repository={} owningIssue={} pullRequest={} reviewThread={} reviewComment={} author={} observedCommit={} path={}", source["repository"], source["owningIssue"]["number"], source["pullRequest"]["number"], source["reviewThread"]["id"], source["reviewComment"]["id"], source["author"], source["observedCommit"], source["findings"][0]["path"]);

    let produced_control = produced;
    let current_path = temporary.path().join("current-pr-state.json");
    let control_path = temporary.path().join("review-control.json");
    let previous_path = temporary.path().join("previous-pr-state.json");
    let admitted_path = temporary.path().join("admitted-pr-state.json");
    fs::write(
        &current_path,
        serde_json::to_vec(&direct_state::pr_snapshot(947, &base, &current, None))?,
    )?;
    fs::write(&control_path, serde_json::to_vec(&produced_control)?)?;
    fs::write(
        &previous_path,
        serde_json::to_vec(&direct_state::pr_snapshot(947, &base, &delta, Some(previous)))?,
    )?;
    let admitted = Command::new(repository.join("scripts/build-pr-state"))
        .args(["--repository-root"])
        .arg(repository)
        .args(["--base-pr-state-file"])
        .arg(&current_path)
        .args(["--review-control-state-file"])
        .arg(&control_path)
        .args(["--previous-pr-state-file"])
        .arg(&previous_path)
        .args(["--output"])
        .arg(&admitted_path)
        .env(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        )
        .output()?;
    assert!(
        admitted.status.success(),
        "build-pr-state must admit the live external source: {}",
        String::from_utf8_lossy(&admitted.stderr)
    );
    let admitted: Value = serde_json::from_slice(&fs::read(admitted_path)?)?;
    assert_eq!(
        admitted["reviewControl"]["post_cap_re_review"]["qualifying_change"]
            ["external_finding"]["reviewComment"]["id"],
        "PRRC_kwDOS6i-_87q4eM0"
    );
    Ok(())
}

#[test]
#[ignore = "requires an authenticated live GitHub read"]
fn producer_rejects_nonexistent_or_mismatched_live_locator() -> TestResult {
    for (pull_request, review_comment) in [(938, "PRRC_nonexistent"), (999_999, "PRRC_kwDOS6i-_87q4eM0")] {
        let temporary = tempfile::tempdir()?;
        let input = temporary.path().join("input.json");
        let output = temporary.path().join("output.json");
        fs::write(
            &input,
            serde_json::to_vec(&json!({
                "control_state": {
                    "schema": "codexy.review-control-state.v1",
                    "profile": "strict",
                    "post_cap_re_review": {"reason": "authenticated_external_finding_repair"}
                },
                "authenticated_external_finding_locator": {
                    "repository": "eunsoogi/codexy",
                    "owningIssue": 937,
                    "pullRequest": pull_request,
                    "reviewThread": "PRRT_kwDOS6i-_86fjYep",
                    "reviewComment": review_comment
                }
            }))?,
        )?;
        let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
            .args(["--produce-review-control", "--input"])
            .arg(&input)
            .args(["--output"])
            .arg(&output)
            .output()?;
        assert!(!result.status.success(), "invalid live locator was accepted");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("authenticated GitHub"),
            "invalid locator must fail at the authenticated source boundary: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!output.exists());
    }
    Ok(())
}

fn git(repository: &std::path::Path, args: &[&str]) -> TestResult<String> {
    let output = Command::new("git").current_dir(repository).args(args).output()?;
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
