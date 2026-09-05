use crate::support::TestResult;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/post_cap_review.rs"]
mod post_cap;
#[path = "support/post_cap_disposition_fixture.rs"]
mod disposition_fixture;

#[test]
fn mixed_delta_block_admits_one_authenticated_finding_disposition() -> TestResult {
    let control = direct_state::post_cap_disposition_control(
        947,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    );
    let state = post_cap::build_pr_state(
        &control,
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    assert_eq!(state["reviewControl"]["terminal_review_count"], 3);
    assert_eq!(
        state["reviewControl"]["post_cap_re_review"]["reason"],
        "authenticated_finding_disposition"
    );
    assert_eq!(
        state["reviewControl"]["post_cap_re_review"]["qualifying_change"]
            ["finding_ids"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    Ok(())
}

#[test]
fn producer_derives_exact_mixed_finding_dispositions_from_live_locators() -> TestResult {
    let control = direct_state::post_cap_disposition_control(
        947,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    );
    let produced = post_cap::produce_disposition(control)?;
    let control = &produced;
    assert_eq!(
        control["post_cap_re_review"]["qualifying_change"]["finding_ids"],
        serde_json::json!([
            "external-source-provenance-not-authenticated",
            "selected-reviewer-policy-mismatch",
            "current-head-ci-incomplete"
        ])
    );
    assert_eq!(
        control["post_cap_re_review"]["qualifying_change"]["finding_disposition"]["sources"]
            ["currentHeadCi"]["complete"],
        true
    );
    assert_eq!(
        control["post_cap_re_review"]["qualifying_change"]["finding_disposition"]["sources"]
            ["maintainerDecision"]["decision"]["actualModel"],
        "gpt-5.6-sol"
    );
    Ok(())
}

#[test]
fn disposition_rejects_reclassification_and_missing_code_repair() -> TestResult {
    let mut reclassified = disposition_control();
    reclassified["post_cap_re_review"]["qualifying_change"]["finding_disposition"]["findings"][0]
        ["requiredDisposition"] = serde_json::json!("current_head_ci_terminal");
    let result = post_cap::run_build(&reclassified, BASE, BASE)?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("reclassifies"));

    let mut no_code_repair = disposition_control();
    no_code_repair["terminal_review_history"][1]["unresolved_findings"][0]["path"] =
        serde_json::json!(".github/workflows/other.yml");
    no_code_repair["post_cap_re_review"]["qualifying_change"]["finding_disposition"]["findings"][0]
        ["path"] = serde_json::json!(".github/workflows/other.yml");
    no_code_repair["post_cap_re_review"]["qualifying_change"]["finding_disposition"]["findings"][0]
        ["requiredDisposition"] = serde_json::json!("current_head_ci_terminal");
    let result = post_cap::run_build(&no_code_repair, BASE, BASE)?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("code-repair evidence"));
    Ok(())
}

#[test]
fn disposition_code_repair_requires_an_exact_path_changing_diff() -> TestResult {
    let mut control = disposition_control();
    control["terminal_review_history"][1]["unresolved_findings"][0]["path"] = serde_json::json!(
        "packages/codexy-runtime/src/validation/review_control/state.rs"
    );
    control["post_cap_re_review"]["qualifying_change"]["finding_disposition"]["findings"][0]
        ["path"] = serde_json::json!(
        "packages/codexy-runtime/src/validation/review_control/state.rs"
    );
    let result = post_cap::run_build(&control, BASE, BASE)?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("not linked to authenticated finding disposition code repair path"));
    Ok(())
}

#[test]
fn disposition_rejects_a_maintainer_body_detached_from_live_pr_refs() -> TestResult {
    let control = disposition_control();
    let result = post_cap::run_build_with_disposition_maintainer(
        &control,
        BASE,
        BASE,
        |pull, base, head| {
            let mut response = disposition_fixture::maintainer_response(pull, pull, base, head);
            response["data"]["repository"]["pullRequest"]["baseRefOid"] =
                serde_json::json!("0000000000000000000000000000000000000000");
            response
        },
    )?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("does not bind its exact repository, issue, PR, refs, or path"));
    Ok(())
}

#[test]
fn disposition_requires_live_source_and_exact_finding_coverage() -> TestResult {
    let mut missing_source = disposition_control();
    missing_source["post_cap_re_review"]["qualifying_change"]
        .as_object_mut()
        .expect("qualifying change")
        .remove("finding_disposition");
    let result = post_cap::run_build(&missing_source, BASE, BASE)?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("finding disposition"));

    let mut reordered = disposition_control();
    reordered["post_cap_re_review"]["qualifying_change"]["finding_ids"] = serde_json::json!([
        "selected-reviewer-policy-mismatch",
        "external-source-provenance-not-authenticated",
        "current-head-ci-incomplete"
    ]);
    let result = post_cap::run_build(&reordered, BASE, BASE)?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("ids do not match"));
    Ok(())
}

fn disposition_control() -> serde_json::Value {
    direct_state::post_cap_disposition_control(
        947,
        FULL_HEAD,
        DELTA_HEAD,
        CURRENT_HEAD,
    )
}

const FULL_HEAD: &str = direct_state::SYNTHETIC_FULL_HEAD;
const DELTA_HEAD: &str = direct_state::SYNTHETIC_DELTA_HEAD;
const CURRENT_HEAD: &str = direct_state::SYNTHETIC_CURRENT_HEAD;
const BASE: &str = direct_state::SYNTHETIC_BASE;
