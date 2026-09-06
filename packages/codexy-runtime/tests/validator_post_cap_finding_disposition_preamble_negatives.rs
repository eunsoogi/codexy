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
