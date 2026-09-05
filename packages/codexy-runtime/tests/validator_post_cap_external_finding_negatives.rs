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
fn external_finding_rejects_stale_identity_and_unrelated_repair() -> TestResult {
    let mut stale = external_control();
    let stale_finding = &mut stale["post_cap_re_review"]["qualifying_change"]["external_finding"];
    stale_finding["observedCommit"] = json!("0000000000000000000000000000000000000000");
    stale_finding["capture"]["raw"]["observedCommit"] =
        json!("0000000000000000000000000000000000000000");
    assert_rejected(stale, BASE, BASE, "stale for the prior delta head")?;

    let mut wrong_path = external_control();
    let wrong_path_finding =
        &mut wrong_path["post_cap_re_review"]["qualifying_change"]["external_finding"];
    wrong_path_finding["findings"][0]["path"] = json!("README.md");
    wrong_path_finding["capture"]["raw"]["findings"][0]["path"] = json!("README.md");
    assert_rejected(wrong_path, BASE, BASE, "not linked to authenticated external finding path")?;

    let mut malformed_path = external_control();
    let malformed_path_finding =
        &mut malformed_path["post_cap_re_review"]["qualifying_change"]["external_finding"];
    malformed_path_finding["findings"][0]["path"] = json!("../state.rs");
    malformed_path_finding["capture"]["raw"]["findings"][0]["path"] = json!("../state.rs");
    assert_rejected(malformed_path, BASE, BASE, "must be repository-relative")?;
    Ok(())
}

#[test]
fn external_finding_rejects_source_mismatch_and_base_change() -> TestResult {
    let mut wrong_repository = external_control();
    let wrong_repository_finding =
        &mut wrong_repository["post_cap_re_review"]["qualifying_change"]["external_finding"];
    wrong_repository_finding["repository"] = json!("other/repository");
    wrong_repository_finding["capture"]["raw"]["repository"] = json!("other/repository");
    assert_rejected(
        wrong_repository,
        BASE,
        BASE,
        "owning issue changes repository identity",
    )?;

    let mut wrong_url = external_control();
    let wrong_url_finding =
        &mut wrong_url["post_cap_re_review"]["qualifying_change"]["external_finding"];
    wrong_url_finding["reviewComment"]["url"] =
        json!("https://github.com/eunsoogi/codexy/pull/938#discussion_r7");
    wrong_url_finding["capture"]["raw"]["reviewComment"]["url"] =
        json!("https://github.com/eunsoogi/codexy/pull/938#discussion_r7");
    assert_rejected(
        wrong_url,
        BASE,
        BASE,
        "review identity is not bound to its canonical URL",
    )?;

    let changed_base = external_control();
    assert_rejected(
        changed_base,
        BASE,
        direct_state::SYNTHETIC_UPDATED_BASE,
        "must not change baseRefOid",
    )?;
    Ok(())
}

#[test]
fn external_finding_requires_clean_delta_and_exact_source_ids() -> TestResult {
    let mut blocked = external_control();
    blocked["terminal_review_history"][1]["terminal_result"] = json!("BLOCK");
    assert_rejected(blocked, BASE, BASE, "requires a clean prior PASS delta")?;

    let mut mismatched_ids = external_control();
    mismatched_ids["post_cap_re_review"]["qualifying_change"]["finding_ids"] =
        json!(["different-finding"]);
    assert_rejected(
        mismatched_ids,
        BASE,
        BASE,
        "finding ids do not bind the external source",
    )?;

    let mut missing_source = external_control();
    missing_source["post_cap_re_review"]["qualifying_change"]
        .as_object_mut()
        .expect("qualifying change")
        .remove("external_finding");
    assert_rejected(missing_source, BASE, BASE, "must bind its source")?;
    Ok(())
}

#[test]
fn external_finding_rejects_unbound_raw_authenticated_capture() -> TestResult {
    let mut missing_raw = external_control();
    missing_raw["post_cap_re_review"]["qualifying_change"]["external_finding"]["capture"]
        .as_object_mut()
        .expect("capture")
        .remove("raw");
    assert_rejected(
        missing_raw,
        BASE,
        BASE,
        "requires raw authenticated capture",
    )?;

    let mut rebound = external_control();
    rebound["post_cap_re_review"]["qualifying_change"]["external_finding"]["reviewThread"]
        ["id"] = json!("PRRT_fake");
    assert_rejected(
        rebound,
        BASE,
        BASE,
        "does not match raw authenticated capture",
    )?;
    Ok(())
}

#[test]
fn external_finding_rejects_caller_forged_legacy_source() -> TestResult {
    let control = external_control();
    let mut forged = pr938_finding(DELTA_HEAD);
    let finding = &mut forged;
    finding["capture"] = json!({
        "provider": "github",
        "method": "graphql",
        "authenticated": true
    });
    finding["reviewThread"]["id"] = json!("PRRT_fake");
    let result = post_cap::produce(&control, &forged, BASE, BASE);
    assert!(result.is_err(), "producer accepted a caller-forged source");
    Ok(())
}

fn external_control() -> Value {
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
    control["post_cap_re_review"]["qualifying_change"]["external_finding"] =
        pr938_finding(DELTA_HEAD);
    control
}

fn assert_rejected(
    control: Value,
    previous_base: &str,
    current_base: &str,
    diagnostic: &str,
) -> TestResult<()> {
    let result = post_cap::run_build(&control, previous_base, current_base)?;
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains(diagnostic),
        "expected {diagnostic:?} in stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}

fn pr938_finding(observed_commit: &str) -> Value {
    let raw = json!({
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
        "author": "chatgpt-codex-connector",
        "observedCommit": observed_commit,
        "findings": [{
            "id": FINDING_ID,
            "path": "packages/codexy-runtime/src/validation/review_control/state.rs"
        }]
    });
    json!({
        "schema": "codexy.review-control-external-finding.v1",
        "capture": {
            "provider": "github",
            "method": "graphql",
            "authenticated": true,
            "raw": raw
        },
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
        "author": "chatgpt-codex-connector",
        "observedCommit": observed_commit,
        "findings": [{
            "id": FINDING_ID,
            "path": "packages/codexy-runtime/src/validation/review_control/state.rs"
        }]
    })
}
