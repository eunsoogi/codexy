use serde_json::{Value, json};

pub(crate) const FINDING_ID: &str = "github-pr938-discussion-r3940672308";

pub(crate) fn pr938_finding(observed_commit: &str) -> Value {
    let response = pr938_response(observed_commit);
    let projection = pr938_projection(observed_commit);
    let raw = json!({"response": response, "projection": projection.clone()});
    let mut source = projection.as_object().expect("projection").clone();
    source.insert("schema".into(), json!("codexy.review-control-external-finding.v1"));
    source.insert("capture".into(), json!({
        "provider": "github",
        "method": "graphql",
        "authenticated": true,
        "raw": raw
    }));
    Value::Object(source)
}

pub(crate) fn pr938_response(observed_commit: &str) -> Value {
    let repository = "eunsoogi/codexy";
    let pull_url = "https://github.com/eunsoogi/codexy/pull/938";
    let issue_url = "https://github.com/eunsoogi/codexy/issues/937";
    let comment_url = format!("{pull_url}#discussion_r3940672308");
    let path = "packages/codexy-runtime/src/validation/review_control/state.rs";
    let pull = json!({"number":938,"url":pull_url,"repository":{"nameWithOwner":repository}});
    let comment_node = json!({
        "id":"PRRC_kwDOS6i-_87q4eM0","databaseId":3940672308u64,"url":comment_url,
        "author":{"login":"chatgpt-codex-connector"},"commit":{"oid":observed_commit},"path":path
    });
    let comment = json!({
        "__typename":"PullRequestReviewComment","id":"PRRC_kwDOS6i-_87q4eM0",
        "databaseId":3940672308u64,"url":comment_url,
        "author":{"login":"chatgpt-codex-connector"},"commit":{"oid":observed_commit},
        "path":path,"pullRequest":pull
    });
    json!({"data":{
        "repository":{
            "pullRequest":{
                "number":938,"url":pull_url,"repository":{"nameWithOwner":repository},
                "closingIssuesReferences":{"nodes":[{"number":937,"url":issue_url}],"pageInfo":{"hasNextPage":false}}
            },
            "issue":{"number":937,"url":issue_url,"repository":{"nameWithOwner":repository}}
        },
        "thread":{
            "__typename":"PullRequestReviewThread","id":"PRRT_kwDOS6i-_86fjYep","path":path,
            "pullRequest":pull,
            "comments":{"nodes":[comment_node],"pageInfo":{"hasNextPage":false}}
        },
        "comment":comment
    }})
}

fn pr938_projection(observed_commit: &str) -> Value {
    let repository = "eunsoogi/codexy";
    let pull_url = "https://github.com/eunsoogi/codexy/pull/938";
    let issue_url = "https://github.com/eunsoogi/codexy/issues/937";
    let comment_url = format!("{pull_url}#discussion_r3940672308");
    let path = "packages/codexy-runtime/src/validation/review_control/state.rs";
    json!({
        "repository":repository,
        "owningIssue":{"repository":repository,"number":937,"url":issue_url,"association":"closing-issue-reference"},
        "pullRequest":{"repository":repository,"number":938,"url":pull_url},
        "reviewThread":{"id":"PRRT_kwDOS6i-_86fjYep","url":comment_url},
        "reviewComment":{"id":"PRRC_kwDOS6i-_87q4eM0","databaseId":3940672308u64,"url":comment_url},
        "author":"chatgpt-codex-connector",
        "observedCommit":observed_commit,
        "findings":[{"id":FINDING_ID,"path":path}]
    })
}
