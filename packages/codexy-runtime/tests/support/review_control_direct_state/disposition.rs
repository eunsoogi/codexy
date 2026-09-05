use serde_json::{Value, json};

use super::{SYNTHETIC_DISPOSITION_EVIDENCE, post_cap_control_with_findings};

pub(crate) fn post_cap_disposition_control(
    issue_number: u64,
    full_head: &str,
    delta_head: &str,
    current_head: &str,
) -> Value {
    let mut control = post_cap_control_with_findings(
        issue_number,
        full_head,
        delta_head,
        current_head,
        "authenticated_finding_disposition",
        SYNTHETIC_DISPOSITION_EVIDENCE,
        "BLOCK",
        json!([
            {
                "id": "external-source-provenance-not-authenticated",
                "path": "packages/codexy-runtime/src/validation/review_control/external_finding/capture.rs"
            },
            {
                "id": "selected-reviewer-policy-mismatch",
                "path": "plugins/codexy/agents/codexy-sentinel.toml"
            },
            {
                "id": "current-head-ci-incomplete",
                "path": ".github/workflows/bootstrap-package.yml"
            }
        ]),
        json!([
            "external-source-provenance-not-authenticated",
            "selected-reviewer-policy-mismatch",
            "current-head-ci-incomplete"
        ]),
    );
    control["post_cap_re_review"]["qualifying_change"]["finding_disposition"] = json!({
        "schema": "codexy.review-control-finding-disposition.v1",
        "locator": {
            "repository": "eunsoogi/codexy",
            "owningIssue": issue_number,
            "pullRequest": issue_number,
            "maintainerComment": 5554573060u64
        },
        "findings": [
            {
                "id": "external-source-provenance-not-authenticated",
                "path": "packages/codexy-runtime/src/validation/review_control/external_finding/capture.rs",
                "requiredDisposition": "code_repair"
            },
            {
                "id": "selected-reviewer-policy-mismatch",
                "path": "plugins/codexy/agents/codexy-sentinel.toml",
                "requiredDisposition": "maintainer_accepted_policy_difference"
            },
            {
                "id": "current-head-ci-incomplete",
                "path": ".github/workflows/bootstrap-package.yml",
                "requiredDisposition": "current_head_ci_terminal"
            }
        ],
        "sources": {
            "currentHeadCi": {},
            "maintainerDecision": {}
        }
    });
    control
}
