use serde_json::json;

use super::super::live::Locator;
use super::project_response;

#[test]
fn closing_issue_same_number_from_foreign_repository_is_rejected() {
    let locator = Locator::from_value(&json!({
        "repository": "eunsoogi/codexy",
        "owningIssue": 937,
        "pullRequest": 938,
        "reviewThread": "PRRT_kwDOS6i-_86fjYep",
        "reviewComment": "PRRC_kwDOS6i-_87q4eM0"
    }))
    .expect("valid locator");
    let response = json!({
        "data": {
            "repository": {
                "pullRequest": {
                    "number": 938,
                    "url": "https://github.com/eunsoogi/codexy/pull/938",
                    "repository": {"nameWithOwner": "eunsoogi/codexy"},
                    "closingIssuesReferences": {
                        "nodes": [{
                            "number": 937,
                            "url": "https://github.com/other/codexy/issues/937",
                            "repository": {"nameWithOwner": "other/codexy"}
                        }],
                        "pageInfo": {"hasNextPage": false}
                    }
                },
                "issue": {
                    "number": 937,
                    "url": "https://github.com/eunsoogi/codexy/issues/937",
                    "repository": {"nameWithOwner": "eunsoogi/codexy"}
                }
            }
        }
    });

    let error = project_response(response.as_object().expect("response"), &locator, None)
        .expect_err("a same-number issue from another repository must be rejected");
    assert!(error.contains("not a closing PR reference"), "{error}");
}
