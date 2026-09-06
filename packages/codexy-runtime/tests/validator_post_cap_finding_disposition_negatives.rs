use serde_json::Value;

use crate::support::TestResult;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/post_cap_review.rs"]
mod post_cap;
#[path = "support/post_cap_disposition_fixture.rs"]
mod disposition_fixture;

const BASE: &str = direct_state::SYNTHETIC_BASE;

fn control() -> Value {
    direct_state::post_cap_disposition_control(
        947,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    )
}

#[test]
fn disposition_binds_the_maintainer_decision_to_the_full_reviewer_tuple() -> TestResult {
    let result = post_cap::run_build_with_disposition_maintainer(
        &control(),
        BASE,
        BASE,
        |pull, base, head| {
            let mut response = disposition_fixture::maintainer_response(pull, pull, base, head);
            let body = response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]
                ["body"]
                .as_str()
                .expect("maintainer fixture body")
                .replace("gpt-5.6-sol/xhigh", "gpt-6-astra/xhigh");
            response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]["body"] =
                Value::String(body);
            response
        },
    )?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("reviewer tuple"));
    Ok(())
}

#[test]
fn disposition_rejects_fenced_or_negated_maintainer_copies() -> TestResult {
    let cases = ["fenced", "negated"];
    for case in cases {
        let result = post_cap::run_build_with_disposition_maintainer(
            &control(),
            BASE,
            BASE,
            move |pull, base, head| {
                let mut response = disposition_fixture::maintainer_response(pull, pull, base, head);
                let original = response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]
                    ["body"]
                    .as_str()
                    .expect("maintainer fixture body");
                let body = if case == "fenced" {
                    format!("```markdown\n{original}\n```")
                } else {
                    original.replace(
                        "the retained Sentinel's actual",
                        "not accepted: the retained Sentinel's actual",
                    )
                };
                response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]["body"] =
                    Value::String(body);
                response
            },
        )?;
        assert!(!result.status.success(), "{case} copy must be rejected");
    }
    Ok(())
}

#[test]
fn disposition_rejects_contradictory_text_before_scope_heading() -> TestResult {
    let result = post_cap::run_build_with_disposition_maintainer(
        &control(),
        BASE,
        BASE,
        |pull, base, head| {
            let mut response = disposition_fixture::maintainer_response(pull, pull, base, head);
            let original = response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]
                ["body"]
                .as_str()
                .expect("maintainer fixture body");
            let body = original.replace(
                "## Maintainer disposition recorded by the release orchestrator",
                "This disposition is revoked and must not be accepted.\n\n## Maintainer disposition recorded by the release orchestrator",
            );
            response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]["body"] =
                Value::String(body);
            response
        },
    )?;
    assert!(
        !result.status.success(),
        "contradictory pre-heading text must be rejected"
    );
    Ok(())
}

#[test]
fn disposition_rejects_operational_revocation_inside_bounded_preamble() -> TestResult {
    let result = post_cap::run_build_with_disposition_maintainer(
        &control(),
        BASE,
        BASE,
        |pull, base, head| {
            let mut response = disposition_fixture::maintainer_response(pull, pull, base, head);
            let original = response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]
                ["body"]
                .as_str()
                .expect("maintainer fixture body");
            let body = original.replace(
                "differences between the actually used specialist models and the planned 1.7.0 specialist routing are accepted for this milestone.",
                "this disposition is revoked; only unrelated model differences are accepted for this milestone.",
            );
            response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]["body"] =
                Value::String(body);
            response
        },
    )?;
    assert!(
        !result.status.success(),
        "operative revocation inside the preamble must be rejected"
    );
    Ok(())
}

#[test]
fn disposition_rejects_a_contradictory_accepted_difference_suffix() -> TestResult {
    let result = post_cap::run_build_with_disposition_maintainer(
        &control(),
        BASE,
        BASE,
        |pull, base, head| {
            let mut response = disposition_fixture::maintainer_response(pull, pull, base, head);
            let original = response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]
                ["body"]
                .as_str()
                .expect("maintainer fixture body");
            let body = original.replace(
                "execution may stand despite the planned newer model routing.",
                "execution may stand despite the planned newer model routing. This is not accepted.",
            );
            response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]["body"] =
                Value::String(body);
            response
        },
    )?;
    assert!(
        !result.status.success(),
        "contradictory accepted-difference suffix must be rejected"
    );
    Ok(())
}

#[test]
fn disposition_rejects_an_unexpected_operative_bullet() -> TestResult {
    let result = post_cap::run_build_with_disposition_maintainer(
        &control(),
        BASE,
        BASE,
        |pull, base, head| {
            let mut response = disposition_fixture::maintainer_response(pull, pull, base, head);
            let original = response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]
                ["body"]
                .as_str()
                .expect("maintainer fixture body");
            let body = original.replace(
                "Scope of this disposition:\n",
                "Scope of this disposition:\n- Additional note: this is an operative statement.\n",
            );
            response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]["body"] =
                Value::String(body);
            response
        },
    )?;
    assert!(
        !result.status.success(),
        "unexpected operative bullet must be rejected"
    );
    Ok(())
}

#[test]
fn disposition_rejects_empty_incomplete_unsupported_and_non_success_ci_rollups() -> TestResult {
    for case in ["empty", "incomplete", "unsupported", "pending", "failed", "cancelled"] {
        let result = post_cap::run_build_with_disposition_ci(
            &control(),
            BASE,
            BASE,
            move |_pull, _base, _head, sources| {
                match case {
                    "empty" => sources.pull_request["statusCheckRollup"] = serde_json::json!([]),
                    "incomplete" => sources.pull_request["statusCheckRollup"] = serde_json::json!([{"__typename":"CheckRun"}]),
                    "unsupported" => sources.pull_request["statusCheckRollup"] = serde_json::json!([{"__typename":"StatusContext"}]),
                    "pending" => sources.pull_request["statusCheckRollup"][0]["status"] = serde_json::json!("IN_PROGRESS"),
                    "failed" => sources.pull_request["statusCheckRollup"][0]["conclusion"] = serde_json::json!("FAILURE"),
                    "cancelled" => sources.pull_request["statusCheckRollup"][0]["conclusion"] = serde_json::json!("CANCELLED"),
                    _ => unreachable!(),
                }
            },
        )?;
        assert!(!result.status.success(), "{case} CI rollup must be rejected");
    }
    Ok(())
}

#[test]
fn disposition_rejects_missing_required_or_pending_expected_check_runs() -> TestResult {
    let cases = [
        "missing-required",
        "pending-expected",
        "pending-suite",
        "missing-inventory",
    ];
    for case in cases {
        let result = post_cap::run_build_with_disposition_ci(
            &control(),
            BASE,
            BASE,
            move |_pull, _base, _head, sources| {
                match case {
                    "missing-required" => {
                        sources.required_status_checks = serde_json::json!({
                            "required_status_checks": {
                                "contexts": ["Not yet registered"],
                                "checks": []
                            }
                        });
                    }
                    "pending-expected" => {
                        sources.expected_check_runs[0]["check_runs"][0]["status"] =
                            serde_json::json!("in_progress");
                    }
                    "pending-suite" => {
                        sources.check_suites[0]["check_suites"][0]["status"] =
                            serde_json::json!("in_progress");
                    }
                    "missing-inventory" => {
                        sources.expected_check_runs = serde_json::json!([{
                            "total_count": 1,
                            "check_runs": []
                        }]);
                    }
                    _ => unreachable!(),
                }
            },
        )?;
        assert!(!result.status.success(), "{case} CI evidence must remain unproved");
    }
    Ok(())
}
