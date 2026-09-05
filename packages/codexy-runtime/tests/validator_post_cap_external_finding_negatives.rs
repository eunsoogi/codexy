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
fn external_finding_rejects_stale_identity_and_unrelated_repair() -> TestResult {
    let mut stale = external_control();
    let stale_finding = &mut stale["post_cap_re_review"]["qualifying_change"]["external_finding"];
    stale_finding["observedCommit"] = json!("0000000000000000000000000000000000000000");
    stale_finding["capture"]["raw"]["projection"]["observedCommit"] =
        json!("0000000000000000000000000000000000000000");
    stale_finding["capture"]["raw"]["response"]["data"]["thread"]["comments"]["nodes"][0]
        ["commit"]["oid"] = json!("0000000000000000000000000000000000000000");
    stale_finding["capture"]["raw"]["response"]["data"]["comment"]["commit"]["oid"] =
        json!("0000000000000000000000000000000000000000");
    assert_rejected(stale, BASE, BASE, "does not match live GitHub source")?;

    let mut wrong_path = external_control();
    let wrong_path_finding =
        &mut wrong_path["post_cap_re_review"]["qualifying_change"]["external_finding"];
    wrong_path_finding["findings"][0]["path"] = json!("README.md");
    wrong_path_finding["capture"]["raw"]["projection"]["findings"][0]["path"] = json!("README.md");
    wrong_path_finding["capture"]["raw"]["response"]["data"]["thread"]["path"] =
        json!("README.md");
    wrong_path_finding["capture"]["raw"]["response"]["data"]["thread"]["comments"]["nodes"][0]
        ["path"] = json!("README.md");
    wrong_path_finding["capture"]["raw"]["response"]["data"]["comment"]["path"] =
        json!("README.md");
    assert_rejected(wrong_path, BASE, BASE, "does not match live GitHub source")?;

    let mut malformed_path = external_control();
    let malformed_path_finding =
        &mut malformed_path["post_cap_re_review"]["qualifying_change"]["external_finding"];
    malformed_path_finding["findings"][0]["path"] = json!("../state.rs");
    malformed_path_finding["capture"]["raw"]["projection"]["findings"][0]["path"] =
        json!("../state.rs");
    malformed_path_finding["capture"]["raw"]["response"]["data"]["thread"]["path"] =
        json!("../state.rs");
    malformed_path_finding["capture"]["raw"]["response"]["data"]["thread"]["comments"]["nodes"][0]
        ["path"] = json!("../state.rs");
    malformed_path_finding["capture"]["raw"]["response"]["data"]["comment"]["path"] =
        json!("../state.rs");
    assert_rejected(malformed_path, BASE, BASE, "must be repository-relative")?;
    Ok(())
}

#[test]
fn external_finding_rejects_source_mismatch_and_base_change() -> TestResult {
    let mut wrong_repository = external_control();
    let wrong_repository_finding =
        &mut wrong_repository["post_cap_re_review"]["qualifying_change"]["external_finding"];
    wrong_repository_finding["repository"] = json!("other/repository");
    assert_rejected(
        wrong_repository,
        BASE,
        BASE,
        "does not match raw authenticated projection",
    )?;

    let mut wrong_url = external_control();
    let wrong_url_finding =
        &mut wrong_url["post_cap_re_review"]["qualifying_change"]["external_finding"];
    wrong_url_finding["reviewComment"]["url"] =
        json!("https://github.com/eunsoogi/codexy/pull/938#discussion_r7");
    assert_rejected(
        wrong_url,
        BASE,
        BASE,
        "does not match raw authenticated projection",
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
fn external_finding_rejects_matching_persisted_forgery_against_live_read() -> TestResult {
    let mut forged = external_control();
    let finding = &mut forged["post_cap_re_review"]["qualifying_change"]["external_finding"];
    finding["author"] = json!("caller-controlled");
    finding["capture"]["raw"]["projection"]["author"] = json!("caller-controlled");
    finding["capture"]["raw"]["response"]["data"]["comment"]["author"]["login"] =
        json!("caller-controlled");
    finding["capture"]["raw"]["response"]["data"]["thread"]["comments"]["nodes"][0]
        ["author"]["login"] = json!("caller-controlled");
    assert_rejected(forged, BASE, BASE, "does not match live GitHub source")?;
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
        "does not match raw authenticated projection",
    )?;
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
