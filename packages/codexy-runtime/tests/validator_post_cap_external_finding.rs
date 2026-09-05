use std::{fs, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;

#[path = "support/post_cap_review.rs"]
mod post_cap;

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
fn producer_normalizes_pr938_finding_and_preserves_prior_history() -> TestResult {
    let finding_id = "github-pr938-discussion-r3940672308";
    let control = direct_state::post_cap_control_with_findings(
        947,
        FULL_HEAD,
        DELTA_HEAD,
        CURRENT_HEAD,
        "authenticated_external_finding_repair",
        EVIDENCE,
        "PASS",
        json!([]),
        json!([]),
    );
    let produced = post_cap::produce(&control, &pr938_finding(DELTA_HEAD), BASE, BASE)?;
    assert_eq!(produced["terminal_review_count"], 3);
    assert_eq!(produced["terminal_review_history"].as_array().map(Vec::len), Some(3));
    assert_eq!(
        produced["terminal_review_history"][0]["kind"],
        "full"
    );
    assert_eq!(
        produced["terminal_review_history"][1]["kind"],
        "delta"
    );
    assert_eq!(
        produced["post_cap_re_review"]["qualifying_change"]["finding_ids"],
        json!([finding_id])
    );
    assert_eq!(
        produced["post_cap_re_review"]["qualifying_change"]["external_finding"]
            ["reviewComment"]["databaseId"],
        3940672308u64
    );
    Ok(())
}

#[test]
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
    control["post_cap_re_review"]["qualifying_change"]["external_finding"] =
        pr938_finding(&delta);
    let previous = direct_state::post_cap_prior(&control);
    let base = git(repository, &["rev-parse", "0c82aedc4748cb40cacaecb08a946b2a8628f8ab^"])?;
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "authenticated_external_finding": pr938_finding(&delta),
            "current_pr_state": direct_state::pr_snapshot(947, &base, &current, None),
            "previous_pr_state": direct_state::pr_snapshot(947, &base, &delta, Some(previous))
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

fn pr938_finding(observed_commit: &str) -> Value {
    json!({
        "schema": "codexy.review-control-external-finding.v1",
        "capture": {"provider": "github", "method": "graphql", "authenticated": true},
        "repository": "eunsoogi/codexy",
        "owningIssue": {
            "repository": "eunsoogi/codexy",
            "number": 937,
            "url": "https://github.com/eunsoogi/codexy/issues/937",
            "association": "linked-issue-reference"
        },
        "pullRequest": {
            "repository": "eunsoogi/codexy",
            "number": 938,
            "url": "https://github.com/eunsoogi/codexy/pull/938"
        },
        "reviewThread": {
            "id": "PRRT_kwDOS6i-_86fjYep",
            "url": "https://github.com/eunsoogi/codexy/pull/938#discussion_r3940672308"
        },
        "reviewComment": {
            "id": "PRRC_kwDOS6i-_87q4eM0",
            "databaseId": 3940672308u64,
            "url": "https://github.com/eunsoogi/codexy/pull/938#discussion_r3940672308"
        },
        "author": "chatgpt-codex-connector[bot]",
        "observedCommit": observed_commit,
        "findings": [{
            "id": "github-pr938-discussion-r3940672308",
            "path": "packages/codexy-runtime/src/validation/review_control/state.rs"
        }]
    })
}
