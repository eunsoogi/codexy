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
fn disposition_rejects_empty_incomplete_unsupported_and_non_success_ci_rollups() -> TestResult {
    for case in ["empty", "incomplete", "unsupported", "pending", "failed", "cancelled"] {
        let result = post_cap::run_build_with_disposition_ci(
            &control(),
            BASE,
            BASE,
            move |pull, base, head| {
                let mut response = disposition_fixture::ci_response(pull, base, head);
                match case {
                    "empty" => response["statusCheckRollup"] = serde_json::json!([]),
                    "incomplete" => response["statusCheckRollup"] = serde_json::json!([{"__typename":"CheckRun"}]),
                    "unsupported" => response["statusCheckRollup"] = serde_json::json!([{"__typename":"StatusContext"}]),
                    "pending" => response["statusCheckRollup"][0]["status"] = serde_json::json!("IN_PROGRESS"),
                    "failed" => response["statusCheckRollup"][0]["conclusion"] = serde_json::json!("FAILURE"),
                    "cancelled" => response["statusCheckRollup"][0]["conclusion"] = serde_json::json!("CANCELLED"),
                    _ => unreachable!(),
                }
                response
            },
        )?;
        assert!(!result.status.success(), "{case} CI rollup must be rejected");
    }
    Ok(())
}
