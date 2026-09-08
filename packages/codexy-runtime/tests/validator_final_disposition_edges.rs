use std::fs;

use crate::support::TestResult;
use serde_json::json;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/final_disposition.rs"]
mod final_support;

const ISSUE: u64 = 993;

#[test]
fn final_disposition_handoff_rejects_reverted_and_out_of_scope_source_repairs() -> TestResult {
    let base = final_support::repaired_head_control(ISSUE);

    for head in [
        direct_state::SYNTHETIC_REVERTED_CURRENT_HEAD,
        direct_state::SYNTHETIC_OUT_OF_SCOPE_CURRENT_HEAD,
    ] {
        let mut control = base.clone();
        control["final_disposition"]["head_oid"] = json!(head);
        let handoff = final_support::validate_handoff(&control, ISSUE, head)?;
        assert!(
            !handoff.status.success(),
            "direct handoff must reject invalid final tree {head}"
        );
    }
    Ok(())
}

#[test]
fn final_disposition_authority_parser_rejects_ambiguous_bodies() -> TestResult {
    let control = final_support::repaired_head_control(ISSUE);
    let previous = final_support::third_block_predecessor(&control);
    let cases: &[(&str, fn(&str) -> String)] = &[
        ("duplicate scope", |body: &str| {
            format!("{body}\nScope of this disposition:")
        }),
        ("duplicate metadata", |body: &str| {
            body.replacen("- Base: ", "- Base: duplicate\n- Base: ", 1)
        }),
        ("negated field", |body: &str| {
            body.replacen(
                "- Remaining findings: none",
                "- Remaining findings: not none",
                1,
            )
        }),
        ("extra operative bullet", |body: &str| {
            body.replacen(
                "\n- Decision: ",
                "\n- Extra: unapproved\n- Decision: ",
                1,
            )
        }),
        ("fenced stale copy", |body: &str| {
            format!("{body}\n```text\nstale\n```")
        }),
    ];
    for (label, mutation) in cases {
        let output = final_support::produce_with_fixture_mutation(
            &control,
            &previous,
            ISSUE,
            |fixture| {
                let mut response: serde_json::Value =
                    serde_json::from_slice(&fs::read(&fixture.maintainer)?)?;
                let body = response
                    .pointer("/data/repository/pullRequest/comments/nodes/0/body")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("final authority body")?;
                response["data"]["repository"]["pullRequest"]["comments"]["nodes"][0]["body"] =
                    json!(mutation(body));
                fs::write(&fixture.maintainer, serde_json::to_vec(&response)?)?;
                Ok(())
            },
        )?;
        assert!(!output.status.success(), "{label} authority body must be rejected");
    }
    Ok(())
}
